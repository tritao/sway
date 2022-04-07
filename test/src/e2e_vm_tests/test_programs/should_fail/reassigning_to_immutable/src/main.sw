script;

struct T { x: u64 }
struct S { t: T, p: (u64, u64)}

fn main() {
    let s = S { t: T { x: 0 }, p: (5, 5) }; // Non-mutable!!
    
    // All of these should fail
    s.t.x = 777; 
    s.t = T { x: 42 }; 
    s.p = (42, 42);
}
