/// Wrapper Vec<u8> in windows, should optimize with zero-copy.
#[derive(Debug)]
pub struct IoVec(Vec<u8>);

impl IoVec {
    pub fn new(value: &[u8]) -> Self {
        IoVec(value.to_vec())
    }
    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self.0.as_mut_slice()
    }
    pub fn as_vec(&self) -> &Vec<u8> {
        &self.0
    }
    pub fn as_mut_vec(&mut self) -> &mut Vec<u8> {
        &mut self.0
    }
    pub fn modify<'a>(&mut self, value: &'a [u8]) -> Result<(), &'a [u8]> {
        let self_len = self.0.len();
        let value_len = value.len();

        match self_len.cmp(&value_len) {
            std::cmp::Ordering::Equal => self.0 = value.to_vec(),
            _ => return Err(value),
        }

        Ok(())
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
