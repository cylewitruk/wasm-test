use std::cell::RefCell;

use clarity::vm::Value;

#[derive(Debug, Clone)]
struct Stack {
    locals: RefCell<Vec<Value>>
}

impl Stack {
    pub fn new() -> Self {
        Stack {
            locals: Default::default()
        }
    }

    pub fn frame(&self) -> Frame {
        Frame::new(self, None)
    }

    fn tombstone<'a>(&self, frame: &'a Frame<'_>) {
        println!("TOMBSTONE ME!! {:?}", frame);
    }
}

#[derive(Debug, Clone)]
struct Frame<'a> {
    stack: &'a Stack,
    ancestor: Option<&'a Frame<'a>>
}

impl<'a> Frame<'_> {
    pub fn new(stack: &'a Stack, ancestor: Option<&'a Frame<'a>>) -> Frame<'a> {
        Frame {
            stack,
            ancestor: ancestor.map(|frame| frame)
        }
    }

    pub fn frame(&'a self) -> Frame<'a> {
        Frame::new(self.stack, Some(self))
    }
}

impl<'a> Drop for Frame<'_> {
    fn drop(&mut self) {
       self.stack.tombstone(self);
    }
}

struct FrameResult {

}

#[cfg(test)]
mod test {
    use super::Stack;

    #[test]
    fn test() {
        let stack = Stack::new();
        let frame1 = stack.frame();
        let frame2 = frame1.frame();

        eprintln!("stack: {:?}", stack);
        eprintln!("frame1: {:?}", frame1);
        eprintln!("frame2: {:?}", frame2);
    }
}