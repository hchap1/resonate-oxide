use std::thread::JoinHandle;
use std::thread::spawn;
use std::path::PathBuf;

use crossbeam_channel::Sender;
use crossbeam_channel::Receiver;
use crossbeam_channel::unbounded;

use rusqlite::types::Value;
use rusqlite::Connection;
use rusqlite::params_from_iter;
use rusqlite::ParamsFromIter;
use rusqlite::types::ValueRef;

enum ItemStream {
    Value(DatabaseParam),
    Error,
    End
}

enum DatabaseTask {
    Execute(&'static str, DatabaseParams),
    Query(&'static str, DatabaseParams, Sender<ItemStream>)
}

pub enum DatabaseParam {
    String(String),
    Usize(usize),
    Null,
    F46(f64)
}

impl DatabaseParam {
    fn to_sql(&self) -> Value {
        match self {
            Self::String(v) => Value::from(v.to_owned()),
            Self::Usize(v) => Value::from(*v as isize),
            Self::Null => Value::Null,
            Self::F46(v) => Value::Real(*v)
        }
    }
}

pub struct DatabaseParams {
    params: Vec<DatabaseParam>
}

impl DatabaseParams {
    fn to_params(&self) -> ParamsFromIter<Vec<Value>> {
        let params: Vec<Value> = self.params.iter().map(|x| x.to_sql()).collect();
        params_from_iter(params)
    }

    pub fn empty() -> DatabaseParams {
        DatabaseParams {
            params: Vec::new()
        }
    }

    pub fn new(params: Vec<DatabaseParam>) -> DatabaseParams {
        DatabaseParams {
            params
        }
    }
}

pub struct Database {
    handle: JoinHandle<()>,
    task_sender: Sender<DatabaseTask>
}

impl Database {
    pub fn new(root_dir: PathBuf) -> Database {

        let (task_sender, task_receiver) = unbounded();

        Database {
            handle: spawn(move || database_thread(root_dir, task_receiver)),
            task_sender
        }
    }

    /// Spawn an execute, but don't wait around for it to finish. Non-blocking.
    pub fn execute(&self, query: &'static str, params: DatabaseParams) -> Result<(), ()> {
        self.task_sender.send(DatabaseTask::Execute(query, params)).map_err(|_| ())
    }

    pub async fn query_map(&self, query: &'static str, params: DatabaseParams) -> Result<Vec<DatabaseParam>, ()> {
        let (sender, receiver) = unbounded();
        let _ = self.task_sender.send(DatabaseTask::Query(query, params, sender));
        let handle = tokio::task::spawn_blocking(move || {
            let mut values = Vec::new();
            let mut error = false;
            while let Ok(item) = receiver.recv() {
                match item {
                    ItemStream::End => break,
                    ItemStream::Error => { error = true; break },
                    ItemStream::Value(v) => values.push(v)
                };
            }

            (values, error)
        });

        let (values, success) = match handle.await {
            Ok(data) => data,
            Err(_) => return Err(())
        };

        match success {
            true => Ok(values),
            false => Err(())
        }
    }
}

fn database_thread(root_dir: PathBuf, task_receiver: Receiver<DatabaseTask>) {

    let connection = match Connection::open(root_dir.join("data.db")) {
        Ok(connection) => connection,
        Err(_) => return
    };

    loop {
        let current_task = match task_receiver.recv() {
            Ok(task) => task,
            Err(_) => return
        };

        match current_task {
            DatabaseTask::Execute(query, params) => {
                if let Ok(mut statement) = connection.prepare(query) {
                    let _ = statement.execute(params.to_params());
                }
            },
            DatabaseTask::Query(query, params, sender) => {
                let mut statement = match connection.prepare(query) {
                    Ok(statement) => statement,
                    Err(_) => {
                        let _ = sender.send(ItemStream::Error);
                        continue
                    }
                };

                let column_count = statement.column_count();
                let _ = statement.query_map(params.to_params(), |row| {

                    for idx in 0..column_count {
                        let value = match row.get_ref(idx) {
                            Ok(value) => value,
                            Err(_) => continue
                        };

                        let value = match value {
                            ValueRef::Null => DatabaseParam::Null,
                            ValueRef::Integer(i) => DatabaseParam::Usize(i as usize),
                            ValueRef::Real(f) => DatabaseParam::F46(f),
                            ValueRef::Text(s) => DatabaseParam::String(String::from_utf8_lossy(s).into_owned()),
                            ValueRef::Blob(_) => DatabaseParam::Null
                        };

                        let _ = sender.send(ItemStream::Value(value));
                    }

                    Ok(())
                });

                let _ = sender.send(ItemStream::End);
            }
        }
    }
}
