use std::{
    rc::Rc,
    cell::{
        RefCell,
        Ref,
        RefMut
    }
};

pub struct Shared<T>(Rc<RefCell<T>>);

impl<A> Clone for Shared<A> {
    fn clone(&self) -> Shared<A> {
        let value = Rc::clone(&self.0);
        Shared(value)
    }
}

impl<T> From<T> for Shared<T> {
    fn from(value: T) -> Shared<T> {
        Shared::new(value)
    }
}

impl<T> Shared<T> {
    pub fn new(value: T) -> Shared<T> {
        let value = RefCell::new(value);
        let value = Rc::new(value);
        Shared(value)
    }

    pub fn borrow(&self) -> Ref<T> {
        self.0.borrow()
    }

    pub fn borrow_mut(&self) -> RefMut<T> {
        self.0.borrow_mut()
    }

    pub fn update(&self, u: fn(RefMut<T>)) {
        u(self.borrow_mut())
    }

    pub fn with<R, F: FnOnce(&mut T) -> R>(&self, u: F) -> R {
        u(&mut *self.borrow_mut())
    }
}
