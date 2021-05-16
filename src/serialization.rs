use std::convert::TryInto;

pub struct Loader {
    offset: usize,
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
    offset: usize,
}

impl Storer {
    pub fn new() -> Storer {
        Storer { offset: 0 }
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

    pub fn store_array_u16(&mut self, bytes: &mut [u8], src: &[u16]) {
        for v in src {
            bytes[self.offset..self.offset + 2].copy_from_slice(&v.to_le_bytes()[..]);
            self.offset += 2;
        }
    }

    pub fn load_array_f32(&mut self, bytes: &mut [u8], src: &[f32]) {
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
