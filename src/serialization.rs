use std::convert::TryInto;

pub struct Loader {
    pub offset: usize,
}

impl Loader {
    pub fn new() -> Loader {
        Loader { offset: 0 }
    }

    pub fn load_u16(&mut self, bytes: &[u8]) -> u16 {
        let out = u16::from_le_bytes(bytes[self.offset..self.offset + 2].try_into().unwrap());
        self.offset += 2;
        out
    }

    pub fn load_u32(&mut self, bytes: &[u8]) -> u32 {
        let out = u32::from_le_bytes(bytes[self.offset..self.offset + 4].try_into().unwrap());
        self.offset += 4;
        out
    }

    pub fn load_f32(&mut self, bytes: &[u8]) -> f32 {
        let out = f32::from_le_bytes(bytes[self.offset..self.offset + 4].try_into().unwrap());
        self.offset += 4;
        out
    }

    pub fn load_array_u16(&mut self, bytes: &[u8], count: usize) -> Vec<u16> {
        (0..count)
            .map(|_| {
                let out =
                    u16::from_le_bytes(bytes[self.offset..self.offset + 2].try_into().unwrap());
                self.offset += 2;
                out
            })
            .collect()
    }

    pub fn load_array_f32(&mut self, bytes: &[u8], count: usize) -> Vec<f32> {
        (0..count)
            .map(|_| {
                let out =
                    f32::from_le_bytes(bytes[self.offset..self.offset + 4].try_into().unwrap());
                self.offset += 4;
                out
            })
            .collect()
    }
}

impl Default for Loader {
    fn default() -> Self {
        Loader::new()
    }
}

pub struct Storer {
    pub offset: usize,
}

impl Storer {
    pub fn new() -> Storer {
        Storer { offset: 0 }
    }

    pub fn store_u8(&mut self, bytes: &mut [u8], v: u8) {
        bytes[self.offset] = v;
        self.offset += 1;
    }

    pub fn store_u16(&mut self, bytes: &mut [u8], v: u16) {
        bytes[self.offset..self.offset + 2].copy_from_slice(&v.to_le_bytes()[..]);
        self.offset += 2;
    }

    pub fn store_u32(&mut self, bytes: &mut [u8], v: u32) {
        bytes[self.offset..self.offset + 4].copy_from_slice(&v.to_le_bytes()[..]);
        self.offset += 4;
    }

    pub fn store_f32(&mut self, bytes: &mut [u8], v: f32) {
        bytes[self.offset..self.offset + 4].copy_from_slice(&v.to_le_bytes()[..]);
        self.offset += 4;
    }

    pub fn store_array_u8(&mut self, bytes: &mut [u8], src: &[u8]) {
        bytes[self.offset..self.offset + src.len()].copy_from_slice(src);
        self.offset += src.len();
    }

    pub fn store_array_u16(&mut self, bytes: &mut [u8], src: &[u16]) {
        for v in src {
            bytes[self.offset..self.offset + 2].copy_from_slice(&v.to_le_bytes()[..]);
            self.offset += 2;
        }
    }

    pub fn store_array_f32(&mut self, bytes: &mut [u8], src: &[f32]) {
        for v in src {
            bytes[self.offset..self.offset + 4].copy_from_slice(&v.to_le_bytes()[..]);
            self.offset += 4;
        }
    }
}

impl Default for Storer {
    fn default() -> Self {
        Storer::new()
    }
}

pub struct StorerVec {
    pub v: Vec<u8>,
}

impl StorerVec {
    pub fn new() -> StorerVec {
        StorerVec { v: Vec::new() }
    }

    pub fn store_u8(&mut self, v: u8) {
        self.v.push(v);
    }

    pub fn store_u16(&mut self, v: u16) {
        self.v.extend_from_slice(&v.to_le_bytes()[..]);
    }

    pub fn store_u32(&mut self, v: u32) {
        self.v.extend_from_slice(&v.to_le_bytes()[..]);
    }

    pub fn store_f32(&mut self, v: f32) {
        self.v.extend_from_slice(&v.to_le_bytes()[..]);
    }

    pub fn store_array_u8(&mut self, src: &[u8]) {
        self.v.extend_from_slice(src);
    }

    pub fn store_array_u16(&mut self, src: &[u16]) {
        for v in src {
            self.v.extend_from_slice(&v.to_le_bytes()[..]);
        }
    }

    pub fn store_array_f32(&mut self, src: &[f32]) {
        for v in src {
            self.v.extend_from_slice(&v.to_le_bytes()[..]);
        }
    }
}

impl Default for StorerVec {
    fn default() -> Self {
        StorerVec::new()
    }
}
