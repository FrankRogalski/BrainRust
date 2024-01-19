use std::env::args;
use std::iter::Iterator;
use std::fs;
use std::str::Chars;
use std::collections::VecDeque;
use console::Term;
use std::iter::Peekable;
use std::iter::repeat;
use std::io::stdout;
use std::io::Write;

struct Lexer<'a> {
    bf_code: Chars<'a>,
    line: usize,
    chr: usize
}

impl<'a> Lexer<'a> {
    fn new(bf_code: Chars<'a>) -> Self {
        Lexer { bf_code, line: 1, chr: 0}
    }
}

impl Iterator for Lexer<'_> {
    type Item = char;
    fn next(&mut self) -> Option<char> {
        while let Some(chr) = self.bf_code.next() {
            if chr == '\n' {
                self.line += 1;
                self.chr = 1;
            } else {
                self.chr += 1;
            }
            if ".,<>[]+-".contains(chr) {
                    return Some(chr);
            }
        }
        Option::None
    }
}
#[derive(Debug)]
enum Ops {
    Add(usize),
    Sub(usize),
    Left(usize),
    Right(usize),
    Read(usize),
    Write(usize),
    JmpIfZero(usize),
    JmpIfNeZero(usize)
}

fn count_matches(chr: char, lexer: &mut Peekable<Lexer>) -> usize {
    let mut i = 1;
    while let Some(_) = lexer.next_if(|x| *x == chr) {
        i += 1;
    }
    i
}

fn parse_code() -> Vec<Ops> {
    let file_path = args().nth(1).expect("No File path supplied");
    let bf_file = fs::read_to_string(file_path).expect("File not found");
    let mut lexer = Lexer::new(bf_file.chars()).peekable();
    let mut ops: Vec<Ops> = vec![];
    let mut callstack = vec![];
    while let Some(chr) = lexer.next() {
        let op = match chr {
            '+' => Ops::Add(count_matches(chr, &mut lexer)),
            '-' => Ops::Sub(count_matches(chr, &mut lexer)),
            '<' => Ops::Left(count_matches(chr, &mut lexer)),
            '>' => Ops::Right(count_matches(chr, &mut lexer)),
            ',' => Ops::Read(count_matches(chr, &mut lexer)),
            '.' => Ops::Write(count_matches(chr, &mut lexer)),
            '[' => {
                callstack.push(ops.len());
                Ops::JmpIfZero(0)
            },
            ']' => {
                let matching = callstack.pop()
                    .expect("unmatched closing bracket");
                let jump_to = ops.len();
                if let Some(Ops::JmpIfZero(jmp)) = ops.get_mut(matching) {
                    *jmp = jump_to;
                }
                Ops::JmpIfNeZero(matching)
            },
            _ => panic!("the Lexer fucked up")
        };
        ops.push(op);
    }
    assert!(callstack.is_empty(), "unmatched opening bracket");
    ops
}

fn interpret(ops: &Vec<Ops>) {
    let term = Term::stdout();
    let mut stdout = stdout();
    let mut instruction: usize = 0;
    let mut head: usize = 0;
    let mut memory: VecDeque<u8> = VecDeque::new();
    memory.push_back(0);
    
    while instruction < ops.len() {
        let op = ops.get(instruction).expect("Error parsing the code");
        match op {
            Ops::Add(num) => memory[head] = memory[head].wrapping_add(*num as u8),
            Ops::Sub(num) => memory[head] = memory[head].wrapping_sub(*num as u8),
            Ops::Left(num) => {
                let new_index = head as isize - *num as isize;
                if new_index < 0 {
                    for _ in 0..new_index.abs() {
                        memory.push_front(0);
                        head = 0;
                    }
                } else {
                    head = new_index as usize;
                }
            },
            Ops::Right(num) => {
                head += num;
                if head >= memory.len() {
                    let loops = head - memory.len();
                    for _ in 0..=loops {
                        memory.push_back(0);
                    }
                }
            },
            Ops::Read(num) => {
                let _ = stdout.flush();
                for _ in 0..*num {
                    memory[head] = term.read_char()
                        .expect("not able to read char") as u8;
                }
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
        }
        instruction += 1;
    }
}

fn main() {
    let ops = parse_code();
    interpret(&ops);
}
