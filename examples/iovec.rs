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
}

fn main() {
    let mut bytes = vec![
        (0..16).collect::<Vec<u8>>(),
        (16..32).collect::<Vec<u8>>(),
        (32..48).collect::<Vec<u8>>(),
    ];

    let mut c = bytes[2].clone();

    let mut iovecs = bytes
        .iter_mut()
        .map(|b| IoVec::new(b))
        .collect::<Vec<_>>();

    // println!("{:#?}", iovecs);

    let c = c.as_mut_slice();

    let mut iovecs = iovecs
        .iter_mut()
        .map(|i| i.as_mut_slice())
        .collect::<Vec<_>>();

    iovecs[0] = c;

    println!("{iovecs:#?}");
}
