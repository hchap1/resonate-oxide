use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::mem::replace;

pub type AM<T> = Arc<Mutex<T>>;

pub fn sync<T>(obj: T) -> AM<T> { Arc::new(Mutex::new(obj)) }
pub fn desync<T>(obj: &AM<T>) -> MutexGuard<T> { obj.lock().unwrap() }

pub fn consume(string: &mut String) -> String {
    replace(string, String::new())
}
