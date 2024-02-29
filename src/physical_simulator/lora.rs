#[derive(Debug)]


struct ST {
    a: Vec<u8>,
}

fn f2(a: u8, b: u8) {
    let mut st = ST {
        a: vec![a, b],
    };
    f1(&mut st);
    f1(&mut st);
    f1(&mut st);

    println!("{:?}", st);
}


fn f1(b: &mut ST) {
    b.a.push(10);
    println!("{:?}", b);
}

fn main() {
    f2(1, 2);
}