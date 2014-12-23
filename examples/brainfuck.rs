#![feature(macro_rules, slicing_syntax)]
extern crate jit;

use jit::{Context, UncompiledFunction, Label, flags, get};
use std::cell::RefCell;
use std::io;
use std::io::fs::File;
use std::mem;
use std::os;
use std::path::Path;
use std::rc::Rc;

macro_rules! count(
    ($func:ident, $code:ident, $curr:ident) => ({
        let mut amount = 1u;
        while $code.peek() == Some(&$curr) {
            amount += 1;
            $code.next();
        }
        $func.insn_of(&amount)
    })
);

static PROMPT:&'static str = "> ";
type WrappedLoop<'a> = Rc<RefCell<Loop<'a>>>;
struct Loop<'a> {
    start: Label<'a>,
    end: Label<'a>,
    parent: Option<WrappedLoop<'a>>
}
impl<'a> Loop<'a> {
    fn new(func: &UncompiledFunction<'a>, current_loop: Option<WrappedLoop<'a>>) -> Loop<'a> {
        let mut new_loop = Loop {
            start: Label::new(func),
            end: Label::new(func),
            parent: current_loop
        };
        func.insn_label(&mut new_loop.start);
        new_loop
    }
    fn end(&mut self, func: &UncompiledFunction<'a>) -> Option<WrappedLoop<'a>> {
        func.insn_branch(&mut self.start);
        func.insn_label(&mut self.end);
        let mut parent = None;
        mem::swap(&mut parent, &mut self.parent);
        parent
    }
}

fn compile<'a, T:Iterator<char>>(func: &UncompiledFunction<'a>, code: T) {
    let ubyte = get::<u8>();
    let putchar_sig = get::<fn(u8)>();
    let readchar_sig = get::<fn() -> u8>();
    let ref data = func[0];
    let mut current_loop = None;
    let mut code = code.peekable();
    while let Some(c) = code.next() {
        match c {
            '>' => {
                let ref amount = count!(func, code, c);
                let ref new_value = func.insn_add(data, amount);
                func.insn_store(data, new_value);
            },
            '<' => {
                let ref amount = count!(func, code, c);
                let ref new_value = func.insn_sub(data, amount);
                func.insn_store(data, new_value);
            },
            '+' => {
                let ref amount = count!(func, code, c);
                let mut value = func.insn_load_relative(data, 0, ubyte.clone());
                value = func.insn_add(&value, amount);
                value = func.insn_convert(&value, ubyte.clone(), false);
                func.insn_store_relative(data, 0, &value)
            },
            '-' => {
                let ref amount = count!(func, code, c);
                let mut value = func.insn_load_relative(data, 0, ubyte.clone());
                value = func.insn_sub(&value, amount);
                value = func.insn_convert(&value, ubyte.clone(), false);
                func.insn_store_relative(data, 0, &value)
            },
            '.' => {
                extern fn putchar(c: u8) {
                    io::stdout().write_u8(c).unwrap();
                }
                let ref value = func.insn_load_relative(data, 0, ubyte.clone());
                func.insn_call_native1(Some("putchar"), putchar, putchar_sig.clone(), [value], flags::JIT_CALL_NO_THROW);
            },
            ',' => {
                extern fn readchar() -> u8 {
                    io::stdin().read_byte().unwrap()
                }
                let ref value = func.insn_call_native0(Some("readchar"), readchar, readchar_sig.clone(), flags::JIT_CALL_NO_THROW);
                func.insn_store_relative(data, 0, value);
            },
            '[' => {
                let wrapped_loop = Rc::new(RefCell::new(Loop::new(func, current_loop)));
                let ref tmp = func.insn_load_relative(data, 0, ubyte.clone());
                {
                    let mut borrow = wrapped_loop.borrow_mut();
                    func.insn_branch_if_not(tmp, &mut borrow.end);
                }
                current_loop = Some(wrapped_loop);
            },
            ']' => {
                current_loop = if let Some(ref inner_loop) = current_loop {
                    let mut borrow = inner_loop.borrow_mut();
                    borrow.end(func)
                } else {
                    None
                }
            },
            _ => ()
        }
    };
    func.insn_default_return();
}
fn run(ctx: &mut Context, code: &str) {
    let sig = get::<fn(*mut u8)>();
    let func = ctx.build_func(sig, |func| compile(func, code.chars()));
    func.with(|func:extern fn(*mut u8)| {
        let mut data: [u8, ..3000] = unsafe { mem::zeroed() };
        func(data.as_mut_ptr());
    });
}
fn main() {
    let mut ctx = Context::new();
    match os::args().tail() {
        [ref script] => {
            let ref script = Path::new(script[]);
            let contents = File::open(script).unwrap().read_to_string().unwrap();
            run(&mut ctx, contents[]);
        },
        [] => {
            io::print(PROMPT);
            for line in io::stdin().lock().lines() {
                run(&mut ctx, line.unwrap()[]);
                io::print(PROMPT);
            }
        },
        _ => panic!("Invalid args for Brainfuck VM")
    }
}