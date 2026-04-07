use super::Element;
use anyhow::{anyhow, Result};

pub struct Traverser<'a> {
    elements: &'a Vec<&'a Element>,
    traversed: Vec<i32>,
}

impl<'a> Traverser<'a> {
    pub fn new(elements: &'a Vec<&'a Element>) -> Self {
        Self {
            elements,
            traversed: Vec::with_capacity(elements.len()),
        }
    }

    fn cont(elements: &'a Vec<&'a Element>, traversed: Vec<i32>) -> Self {
        Self {
            elements,
            traversed,
        }
    }

    pub fn traverse(&mut self, index: i32) -> Result<()> {
        for idx in self.elements
            .get(index as usize)
            .ok_or(anyhow!("Reference index out of bounds! {:?} -> {}", self.traversed, index))?
            .children()
        {
            if self.traversed.contains(idx) {
                return Err(anyhow!("Traversal loop found! {:?} -> {}", self.traversed, *idx));
            }
            let mut traversed = self.traversed.clone();
            traversed.push(*idx);
            Self::cont(self.elements, traversed).traverse(*idx)?;
        }
        Ok(())
    }
}
