//! A stand-in for the machine's opener, answering in the test's own thread (sw::opener::set_fake): every link it opens
//! is logged (`w.opener.opened`); `none` set, it is a machine where nothing opens a link, and says why. No browser.
use std::cell::RefCell;

#[derive(Default)]
pub struct FakeOpener { pub opened: RefCell<Vec<String>>, pub none: RefCell<Option<String>> }
impl FakeOpener {
    pub fn run(&self, link: &str) -> Result<(), String> {
        if let Some(why) = self.none.borrow().clone() { return Err(why); }
        self.opened.borrow_mut().push(link.to_string());
        Ok(())
    }
}
