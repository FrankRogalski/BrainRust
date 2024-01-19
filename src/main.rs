use std::env::args;
use std::fs;
use std::str::Chars;
use std::collections::VecDeque;
use std::iter::{repeat, Iterator};
use std::io::{stdout, stdin, Write, Read};

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

fn optimize(ops: &mut Vec<Ops>) {
    for i in (0..ops.len()).rev() {
        if ops.len() - i >= 3 {
            // optimize set to zero
            let mut last_ops = ops[i..i + 3].iter();
            if matches!(last_ops.next(), Some(Ops::JmpIfZero(_))) // [
                && matches!(last_ops.next(), Some(Ops::Add(1)) | Some(Ops::Sub(1)))  // + or -
                && matches!(last_ops.next(), Some(Ops::JmpIfNeZero(_))) { // ]
                ops[i] = Ops::ZeroOut;
                ops.remove(i + 1);
                ops.remove(i + 1);
                continue;
            }
        }
    }
}

fn link_jumps(ops: &mut Vec<Ops>) {
    let mut callstack: Vec<usize> = vec![];
    for i in 0..ops.len() {
        let op = ops.get_mut(i).expect("jump linker out of bounds");
        match op {
            Ops::JmpIfZero(_) => callstack.push(i),
            Ops::JmpIfNeZero(val) => {
                let index = callstack.pop().expect("Linker did not find unmatched closing bracket");
                *val = index;
                if let Some(Ops::JmpIfZero(val)) = ops.get_mut(i) {
                    *val = i;
                }
            },
            _ => (),
        }
    }
}

fn interpret(ops: &Vec<Ops>) {
    let mut stdout = stdout();
    let mut stdin = stdin().bytes();
    let mut instruction: usize = 0;
    let mut head: usize = 0;
    let mut memory: VecDeque<u8> = VecDeque::new();
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
                    repeat(0).take(size).for_each(|_| memory.push_front(0));
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
                    repeat(0).take(loops).for_each(|_| memory.push_back(0));
                }
            },
            Ops::Read(num) => {
                let _ = stdout.flush();
                memory[head] = stdin
                    .nth(num - 1)
                    .expect("No input")
                    .expect("Error reading input");
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
        }
        instruction += 1;
    }
}

fn main() {
    let mut ops = parse_code();
    optimize(&mut ops);
    link_jumps(&mut ops);
    interpret(&ops);
}
