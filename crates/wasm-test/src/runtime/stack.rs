use std::{cell::{RefCell, Cell}, ops::Add};

use clarity::vm::Value;

#[derive(Debug, Clone)]
pub struct Stack {
    locals: RefCell<Vec<Option<Value>>>,
    current_frame_id: RefCell<u32>,
    tombstoned_ptrs: RefCell<Vec<i32>>
}

impl Stack {
    #[inline]
    pub fn new() -> Self {
        Stack {
            locals: Default::default(),
            current_frame_id: RefCell::new(100000),
            tombstoned_ptrs: Default::default()
        }
    }

    #[inline]
    pub fn exec(&self, func: impl Fn(Frame)) -> FrameResult {
        let frame = Frame::new(self.next_frame_id(), self, None);
        func(frame);
        
        FrameResult {  }
    }

    #[inline]
    fn next_frame_id(&self) -> u32 {
        let next_frame_id = *self.current_frame_id.borrow_mut();
        self.current_frame_id.replace(next_frame_id + 1);
        next_frame_id
    }

    #[inline]
    fn tombstone<'a>(&self, frame: &'a Frame<'_>) {
        println!("TOMBSTONE ME!! {:?}", frame);
    }

    #[inline]
    pub fn local_push(&self, value: Value) -> i32 {
        let ptr = self.locals.borrow().len() as i32;

        self.locals
            .borrow_mut()
            .push(Some(value));

        ptr
    }

    #[inline]
    pub fn local_drop(&self, ptr: i32) {
        self.locals.borrow_mut()[ptr as usize] = None;
        self.tombstoned_ptrs.borrow_mut().push(ptr);
    }

    #[inline]
    pub fn clear_locals(&self) {
        self.locals.borrow_mut().clear();
        self.tombstoned_ptrs.borrow_mut().clear();
    }
}

#[derive(Debug, Clone)]
pub struct Frame<'a> {
    id: u32,
    stack: &'a Stack,
    ancestor: Option<&'a Frame<'a>>
}

impl<'a> Frame<'_> {
    #[inline]
    pub fn new(id: u32, stack: &'a Stack, ancestor: Option<&'a Frame<'a>>) -> Frame<'a> {
        Frame {
            id,
            stack,
            ancestor: ancestor.map(|frame| frame)
        }
    }

    #[inline]
    pub fn frame(&'a self, func: impl Fn(Frame)) -> FrameResult {
        let frame = Frame::new(self.stack.next_frame_id(), self.stack, Some(self));
        func(frame);
        FrameResult {  }
    }
}

impl<'a> Drop for Frame<'_> {
    #[inline]
    fn drop(&mut self) {
       self.stack.tombstone(self);
    }
}

pub struct FrameResult {

}

#[cfg(test)]
mod test {
    use clarity::vm::Value;
    use super::Stack;

    #[test]
    fn test() {
        let stack = Stack::new();
        let result = stack.exec(|f1| {
            eprintln!("frame1: {:?}", f1);
            f1.frame(|f2| {
                eprintln!("frame2: {:?}", f2);
                stack.local_push(Value::Int(5));
            });
        });

        

        eprintln!("stack: {:?}", stack);
        
    }
}