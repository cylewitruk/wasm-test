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
        println!("Tombstoning frame: {}", frame.id);
        println!("This frame has {} pointers to clean up.", frame.pointers.borrow().len());

        frame.pointers
            .borrow()
            .iter()
            .for_each(|ptr| self.locals.borrow_mut()[*ptr as usize] = None);

        self.tombstoned_ptrs
            .borrow_mut()
            .append(&mut frame.pointers.borrow_mut());
    }

    #[inline]
    pub fn local_push<'a>(&self, value: Value) -> i32 {
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
    ancestor: Option<&'a Frame<'a>>,
    pointers: RefCell<Vec<i32>>
}

impl<'a> Frame<'_> {
    #[inline]
    pub fn new(
        id: u32, 
        stack: &'a Stack, 
        ancestor: Option<&'a Frame<'a>>
    ) -> Frame<'a> {
        Frame {
            id,
            stack,
            ancestor: ancestor.map(|frame| frame),
            pointers: Default::default()
        }
    }

    #[inline]
    pub fn frame(&'a self, func: impl Fn(Frame)) -> FrameResult {
        let frame = Frame::new(self.stack.next_frame_id(), self.stack, Some(self));
        func(frame);
        FrameResult {  }
    }

    #[inline]
    pub fn local_push(&self, value: Value) -> i32 {
        let ptr = self.stack.local_push(value);
        self.pointers.borrow_mut().push(ptr);
        ptr
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
            f1.local_push(Value::Int(1));
            f1.local_push(Value::Int(2));
            f1.local_push(Value::Int(3));
            eprintln!("frame1: {:?}", f1);
            f1.frame(|f2| {
                eprintln!("frame2: {:?}", f2);
                f2.local_push(Value::Int(5));
            });
        });

        

        eprintln!("stack: {:?}", stack);
        
    }
}