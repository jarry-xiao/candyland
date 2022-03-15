use curve25519_dalek::constants;
use curve25519_dalek::edwards::EdwardsPoint;
use curve25519_dalek::scalar::Scalar;
use rand::seq::SliceRandom;
use rand::thread_rng;
use sha2::Sha512;

const MAX_SIZE: usize = 1024;
const MASK: usize = MAX_SIZE - 1;

pub struct Accumulator {
    heads: [EdwardsPoint; MAX_SIZE],
    removals: [Scalar; MAX_SIZE],
    active_index: usize,
    size: usize,
}


impl Accumulator {
    pub fn new() -> Self {
        let acc = Accumulator {
            heads: [constants::ED25519_BASEPOINT_POINT; MAX_SIZE],
            removals: [Scalar::from_bits([0; 32]); MAX_SIZE],
            active_index: 0,
            size: 1,
        };
        acc
    }

    pub fn get(&self) -> EdwardsPoint {
        self.heads[self.active_index]
    }

    pub fn add(&mut self, elem: Scalar) {
        if self.size < MAX_SIZE {
            self.size += 1;
        }
        let prev = self.heads[self.active_index];
        self.active_index += 1;
        self.active_index &= MASK;
        self.heads[self.active_index] = prev * elem;
    }

    pub fn remove(
        &mut self,
        elem: Scalar,
        proof: EdwardsPoint,
        head: EdwardsPoint,
    ) -> Option<Scalar> {
        let mut j = self.active_index;
        for _ in 0..self.size {
            if self.removals[j] == elem {
                println!("Found element {:?} in removal list", elem);
                return None;
            }
            j = if j == 0 { MASK } else { j - 1 };
        }
        let mut i = 0;
        j = self.active_index;
        loop {
            if i == self.size {
                println!("Element {:?} not found in accumulator", elem);
                return None;
            }
            let current = self.heads[j];
            if head != current {
                i += 1;
                j = if j == 0 { MASK } else { j - 1 };
                continue;
            }
            let proposed = current * elem.invert();
            if proposed != proof {
                return None;
            } else {
                self.add(elem.invert());
                self.removals[self.active_index] = elem;
                return Some(elem);
            }
        }
    }
}

fn main() {
    let G = constants::ED25519_BASEPOINT_POINT;
    let mut v = vec![];
    let mut rng = thread_rng();
    let mut A = Accumulator::new();
    for i in 0..4096 {
        let sk = Scalar::random(&mut rng);
        let pk = G * sk;
        let msg = format!("Hello {}", i);
        let elem = Scalar::hash_from_bytes::<Sha512>(msg.as_bytes());
        v.push((sk, pk, msg));
        A.add(elem + sk);
    }

    for _ in 0..1024 {
        let (sk, _pk, msg) = v.choose(&mut rng).unwrap();
        let elem = Scalar::hash_from_bytes::<Sha512>(msg.as_bytes()) + sk;
        let head = A.get();
        match A.remove(elem, head * elem.invert(), head) {
            Some(_) => println!("Removed message: {}", msg),
            None => {}
        }
    }

    println!("{:?}", A.get());
}
