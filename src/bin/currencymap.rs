use std::sync::{Arc,Mutex};
use std::thread;
use std::collections::HashMap;


fn main(){
    let mut map:HashMap<String,String> = HashMap::new();
    let mut arcmap = Arc::new(Mutex::new(map));

    let mut arcmap1= arcmap.clone();
    let mp = arcmap.clone();
    let th1= thread::spawn(move || {
        mp.lock().unwrap().insert("ccc".to_owned(),"ccc".to_owned());
    });

    let th2 = thread::spawn(move || {
        // let mut mp = arcmap1.clone();
        arcmap1.lock().unwrap().insert("bbb".to_owned(),"bbb".to_owned());
    });

    th1.join();
    th2.join();
    println!("{:?}",arcmap);
}