use std::env::args;
use std::fs;
use std::str::Chars;
use std::collections::VecDeque;
use std::iter::{repeat, Iterator};
use std::io::{stdout, stdin, Write, Read, Bytes, Stdin, IsTerminal};
use console::Term;

struct Lexer<'a> {
    bf_code: Chars<'a>,
    current: Option<char>,
}

impl<'a> Lexer<'a> {
    fn new(bf_code: Chars<'a>) -> Self {
        Lexer { bf_code, current: None}
    }
}

impl Iterator for Lexer<'_> {
    type Item = char;
    fn next(&mut self) -> Option<char> {
        while let Some(chr) = self.bf_code.next() {
            if ".,<>[]+-".contains(chr) {
                self.current = Some(chr);
                return Some(chr);
            }
        }
        self.current = None;
        Option::None
    }
}

#[derive(Debug, PartialEq, Clone)]
enum Ops {
    // normal commands
    Add(u8),
    Sub(u8),
    Left(isize),
    Right(usize),
    Read(usize),
    Write(usize),
    JmpIfZero(usize),
    JmpIfNeZero(usize),
    // optimized commands
    ZeroOut,
    AddToSide(isize),
    SubFromSide(isize),
    Mul(u8, isize),
}

fn count_matches(chr: char, lexer: &mut Lexer) -> usize {
    lexer.take_while(|&c| c == chr).count() + 1
}

fn parse_code() -> Vec<Ops> {
    let file_path = args().nth(1).expect("No File path supplied");
    let bf_file = fs::read_to_string(file_path).expect("File not found");
    let mut lexer = Lexer::new(bf_file.chars());
    let mut ops: Vec<Ops> = vec![];
    let mut open_count = 0;
    let mut close_count = 0;
    let _ = lexer.next();
    while let Some(chr) = lexer.current {
        let op = match chr {
            '+' => Ops::Add((count_matches(chr, &mut lexer) % 256) as u8),
            '-' => Ops::Sub((count_matches(chr, &mut lexer) % 256) as u8),
            '<' => Ops::Left(count_matches(chr, &mut lexer) as isize),
            '>' => Ops::Right(count_matches(chr, &mut lexer)),
            ',' => Ops::Read(count_matches(chr, &mut lexer)),
            '.' => Ops::Write(count_matches(chr, &mut lexer)),
            '[' => {
                open_count += 1;
                let _ = lexer.next();
                Ops::JmpIfZero(0)
            },
            ']' => {
                close_count += 1;
                let _ = lexer.next();
                Ops::JmpIfNeZero(0)
            },
            illegal_chr => panic!("received illegal character: {}", illegal_chr),
        };
        ops.push(op);
    }
    if open_count > close_count {
        panic!("unmatched opening bracket");
    } else if open_count < close_count {
        panic!("unmatched closing bracket");
    }
    ops
}

fn sum_chain<'a, I>(ops: &mut I) -> (isize, usize)
        where I: Iterator<Item = &'a Ops> {
    match ops.next() {
        Some(Ops::Add(num)) => {
            let res = sum_chain(ops); 
            (res.0 + *num as isize, res.1 + 1)
        },
        Some(Ops::Sub(num)) => {
            let res = sum_chain(ops);
            (res.0 - *num as isize, res.1 + 1)
        },
        _ => (0, 0),
    }
}

fn optimization_add_sub(ops: &mut Vec<Ops>, i: usize) -> usize {
    let mut last_ops = ops[i..].iter();
    let (sum, count) = sum_chain(&mut last_ops);
    if count >= 2 {
        if sum > 0 {
            ops[i] = Ops::Add((sum % 256) as u8);
        } else if sum < 0 {
            ops[i] = Ops::Sub((-sum % 256) as u8);
        }
        repeat(()).take(count - 1).for_each(|_| { ops.remove(i + 1); });

        if sum == 0 {
            ops.remove(i);
            return count;
        } else {
            return count - 1;
        }
    }
    0
}

fn optimization_set_to_zero(ops: &mut Vec<Ops>, i: usize) -> usize {
    use Ops::*;
    match ops[i..] {
        [JmpIfZero(_), Add(1), JmpIfNeZero(_), ..]
            | [JmpIfZero(_), Sub(1), JmpIfNeZero(_), ..] => {
            ops[i] = ZeroOut;
            ops.remove(i + 1);
            ops.remove(i + 1);
            return 1;
        },
        _ => (),
    }
    0
}

fn optimization_add_sub_mul_div(ops: &mut Vec<Ops>, i: usize) -> usize {
    use Ops::*;
    match ops[i..] {
        [JmpIfZero(_), Sub(1), Right(right), Add(val), Left(left), JmpIfNeZero(_), ..]
            | [JmpIfZero(_), Right(right), Add(val), Left(left), Sub(1), JmpIfNeZero(_), ..] if right as isize == left => {
            ops[i] = if val == 1 { AddToSide(left) } else { Mul(val, left) };
            for _ in 0..5 { ops.remove(i + 1); }
            return 1;
        },
        [JmpIfZero(_), Sub(1), Left(left), Add(val), Right(right), JmpIfNeZero(_), ..]
            | [JmpIfZero(_), Left(left), Add(val), Right(right), Sub(1), JmpIfNeZero(_), ..] if right as isize == left => {
            ops[i] = if val == 1 { AddToSide(-left) } else { Mul(val, -left) };
            for _ in 0..5 { ops.remove(i + 1); }
            return 1;
        },
        [JmpIfZero(_), Sub(1), Left(left), Sub(1), Right(right), JmpIfNeZero(_), ..]
            | [JmpIfZero(_), Left(left), Sub(1), Right(right), Sub(1), JmpIfNeZero(_), ..] if right as isize == left => {
            ops[i] = SubFromSide(-left);
            for _ in 0..5 { ops.remove(i + 1); }
            return 1;
        },
        [JmpIfZero(_), Sub(1), Right(right), Sub(1), Left(left), JmpIfNeZero(_), ..]
            | [JmpIfZero(_), Right(right), Sub(1), Left(left), Sub(1), JmpIfNeZero(_), ..] if right as isize == left => {
            ops[i] = SubFromSide(left);
            for _ in 0..5 { ops.remove(i + 1); }
            return 1;
        },
        _ => (),
    }
    0
}

macro_rules! add_to_total {
    ($count_name:ident, $optimization:expr) => {
        let a = $optimization;
        if a > 0 {
            $count_name += a;
            continue;
        }
    };
}

fn optimize(ops: &mut Vec<Ops>) {
    let mut set_to_zero = 0;
    let mut add_sub = 0;
    let mut add_sub_mul_div = 0;
    for i in (0..ops.len()).rev() {
        add_to_total!(add_sub, optimization_add_sub(ops, i));
        add_to_total!(set_to_zero, optimization_set_to_zero(ops, i));
        add_to_total!(add_sub_mul_div, optimization_add_sub_mul_div(ops, i));
    }
    println!("optimized: removed/combined {} add/sub commands", add_sub);
    println!("optimized: created {} set to zero commands", set_to_zero);
    println!("optimized: created {} additions, subtractions, mutliplications and divisions", add_sub_mul_div);
}

fn link_jumps(ops: &mut Vec<Ops>) {
    let mut callstack: Vec<usize> = vec![];
    for i in 0..ops.len() {
        let op = ops.get_mut(i).expect("jump linker out of bounds");
        match op {
            Ops::JmpIfZero(_) => callstack.push(i),
            Ops::JmpIfNeZero(closed_val) => {
                let open_index = callstack.pop().expect("Linker did not find unmatched closing bracket");
                *closed_val = open_index;
                if let Some(Ops::JmpIfZero(open_val)) = ops.get_mut(open_index) {
                    *open_val = i;
                }
            },
            _ => (),
        }
    }
}

enum TerminalType {
    Terminal(Term),
    Command(Bytes<Stdin>),
}

fn interpret(ops: &Vec<Ops>) {
    let mut stdout = stdout();
    let stdin = stdin();
    let mut stdin: TerminalType = if stdin.is_terminal() {
        TerminalType::Terminal(Term::stdout())
    } else {
        TerminalType::Command(stdin.bytes())
    };
    let mut instruction: usize = 0;
    let mut head: usize = 0;
    let mut memory: VecDeque<u8> = VecDeque::new();
    let mut executed: u64 = 0;
    memory.push_back(0);

    while instruction < ops.len() {
        let op = ops.get(instruction).expect("Error in the interpreter");
        match op {
            Ops::Add(num) => memory[head] = memory[head].wrapping_add(*num),
            Ops::Sub(num) => memory[head] = memory[head].wrapping_sub(*num),
            Ops::Left(num) => {
                let new_index = head as isize - num;
                if new_index < 0 {
                    let size = new_index.abs() as usize;
                    memory.reserve(size);
                    repeat(()).take(size).for_each(|_| memory.push_front(0));
                    head = 0;
                } else {
                    head = new_index as usize;
                }
            },
            Ops::Right(num) => {
                head += num;
                if head >= memory.len() {
                    let loops = head - memory.len() + 1;
                    memory.reserve(loops);
                    repeat(()).take(loops).for_each(|_| memory.push_back(0));
                }
            },
            Ops::Read(num) => match stdin {
                TerminalType::Terminal(ref mut term) => {
                    let _ = stdout.flush();
                    for _ in 0..*num {
                        let input = term.read_char().expect("choose wrong input");
                        print!("{}", input);
                        let _ = stdout.flush();
                        memory[head] = input as u8;
                    }
                },
                TerminalType::Command(ref mut stdin) => 
                    memory[head] = stdin.nth(num - 1)
                        .expect("No input")
                        .expect("Error reading input"),
            },
            Ops::Write(num) => {
                let text = repeat(memory[head] as char)
                    .take(*num)
                    .collect::<String>();
                print!("{}", text);
            },
            Ops::JmpIfZero(num) => if memory[head] == 0 {
                instruction = *num;
            },
            Ops::JmpIfNeZero(num) => if memory[head] != 0 {
                instruction = *num;
            },
            Ops::ZeroOut => memory[head] = 0,
            Ops::AddToSide(offset) => {
                let val = memory[head];
                if offset > 0 && head + offset >= memory.len() {
                    memory.push_back(0);
                } else if offset < 0 && head as isize + offset < 0 {
                    for _ in 0..(head as isize + offset) { memory.push_front(0); }
                    head = offset.abs() as usize;
                }
                memory[head + 1] = memory[head + 1].wrapping_add(val);
                memory[head] = 0;
            },
            Ops::AddToLeft => {
                let val = memory[head];
                if head == 0 {
                    memory.push_front(0);
                    head = 1;
                }
                memory[head - 1] = memory[head - 1].wrapping_add(val);
                memory[head] = 0;
            },
            Ops::SubFromRight => {
                let val = memory[head];
                if head + 1 == memory.len() {
                    memory.push_back(0);
                }
                memory[head + 1] = memory[head + 1].wrapping_sub(val);
                memory[head] = 0;
            },
            Ops::SubFromLeft => {
                let val = memory[head];
                if head == 0 {
                    memory.push_front(0);
                    head = 1;
                }
                memory[head - 1] = memory[head - 1].wrapping_sub(val);
                memory[head] = 0;
            },
            Ops::MulRight(num) => {
                let val = memory[head];
                if head + 1 == memory.len() {
                    memory.push_back(0);
                }
                memory[head + 1] = memory[head + 1].wrapping_add(val.wrapping_mul(*num));
                memory[head] = 0;
            },
            Ops::MulLeft(num) => {
                let val = memory[head];
                if head == 0 {
                    memory.push_front(0);
                    head = 1;
                }
                memory[head - 1] = memory[head - 1].wrapping_add(val.wrapping_mul(*num));
                memory[head] = 0;
            },
        }
        instruction += 1;
        executed += 1;
    }
    println!("\nexecuted {} instructions", executed);
    println!("the program has {} commands", ops.len());
}

fn main() {
    let mut ops = parse_code();
    optimize(&mut ops);
    link_jumps(&mut ops);
    interpret(&ops);
}
