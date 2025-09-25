use super::Element;
use anyhow::{anyhow, Result};

pub struct Traverser<'a> {
    elements: &'a Vec<&'a Element>,
    traversed: Vec<i32>,
}

impl<'a> Traverser<'a> {
    pub fn new(elements: &'a Vec<&'a Element>, traversed: Vec<i32>) -> Self {
        Self {
            elements,
            traversed,
        }
    }

    pub fn traverse(&mut self, index: i32) -> Result<()> {
        for idx in self.elements[index as usize].children() {
            if self.traversed.contains(idx) {
                return Err(anyhow!("Traversal loop found! {:?}", self.traversed));
            }
            let mut traversed = self.traversed.clone();
            traversed.push(*idx);
            Self::new(self.elements, traversed).traverse(*idx)?;
        }
        Ok(())
    }
}
