use std::thread::JoinHandle;
use std::thread::spawn;
use std::path::PathBuf;

use async_channel::Sender;
use async_channel::Receiver;
use async_channel::unbounded;

use rusqlite::types::Value;
use rusqlite::Connection;
use rusqlite::params_from_iter;
use rusqlite::ParamsFromIter;
use rusqlite::types::ValueRef;

use super::error::ResonateError;

#[derive(Debug)]
pub enum ItemStream {
    Value(Vec<DatabaseParam>),
    Error,
    End
}

pub enum InsertMessage {
    Success(usize),
    Error
}

pub enum DatabaseTask {
    Execute(&'static str, DatabaseParams),
    WaitExecute(&'static str, DatabaseParams, Sender<()>),
    Insert(&'static str, DatabaseParams, Sender<InsertMessage>),
    Query(&'static str, DatabaseParams, Sender<ItemStream>),
}

#[derive(Clone, Debug)]
pub enum DatabaseParam {
    String(String),
    Usize(usize),
    Null,
    F64(f64),
}

impl DatabaseParam {
    fn to_sql(&self) -> Value {
        match self {
            Self::String(v) => Value::from(v.to_owned()),
            Self::Usize(v) => Value::from(*v as isize),
            Self::Null => Value::Null,
            Self::F64(v) => Value::Real(*v),
        }
    }
    
    pub fn usize(&self) -> usize {
        if let Self::Usize(v) = self { return *v; }
        panic!("Attempted to get a USIZE from a non-usize value");
    }

    pub fn string(&self) -> String {
        if let Self::String(v) = self { return v.clone(); }
        panic!("Attempted to get a STRING from a non-string value");
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

    pub fn empty() -> DatabaseParams { DatabaseParams { params: Vec::new() } }
    pub fn new(params: Vec<DatabaseParam>) -> DatabaseParams { DatabaseParams { params } }
    pub fn single(param: DatabaseParam) -> DatabaseParams { DatabaseParams { params: vec![param] } }
}

pub struct Database {
    _handle: JoinHandle<()>,
    datalink: DataLink
}

#[derive(Clone)]
pub struct DataLink {
    task_sender: Sender<DatabaseTask>
}

impl DataLink {
    pub fn new(task_sender: Sender<DatabaseTask>) -> DataLink {
        DataLink { task_sender }
    }

    pub fn execute(&self, query: &'static str, params: DatabaseParams) -> Result<(), ()> {
        self.task_sender.send_blocking(DatabaseTask::Execute(query, params)).map_err(|_| ())
    }

    pub async fn execute_and_wait(&self, query: &'static str, params: DatabaseParams) -> Result<(), ()> {
        let (sender, receiver) = unbounded();
        let _ = self.task_sender.send_blocking(DatabaseTask::WaitExecute(query, params, sender));
        match receiver.recv().await {
            Ok(_) => Ok(()),
            Err(_) => Err(())
        }
    }

    /// Execute function with receiver callback intended for insert commands (returns row id)
    pub async fn insert(&self, query: &'static str, params: DatabaseParams) -> Option<usize> {
        let (sender, receiver) = unbounded();
        let _ = self.task_sender.send_blocking(DatabaseTask::Insert(query, params, sender));
        let result = match receiver.recv().await {
            Ok(result) => result,
            Err(_) => return None
        };

        match result { InsertMessage::Success(v) => Some(v), InsertMessage::Error => None }
    }

    pub fn insert_stream(&self, query: &'static str, params: DatabaseParams) -> Receiver<InsertMessage> {
        let (sender, receiver) = unbounded();
        let _ = self.task_sender.send_blocking(DatabaseTask::Insert(query, params, sender));
        receiver
    }

    /// Return a receiver that receives the rows
    pub fn query_stream(&self, query: &'static str, params: DatabaseParams) -> Receiver<ItemStream> {
        let (sender, receiver) = unbounded();
        let _ = self.task_sender.send_blocking(DatabaseTask::Query(query, params, sender));
        receiver
    }

    /// Collect all results, then proceed
    pub async fn query_map(
        &self, query: &'static str, params: DatabaseParams
    ) -> Result<Vec<Vec<DatabaseParam>>, ResonateError> {
        let (sender, receiver) = unbounded();
        let _ = self.task_sender.send_blocking(DatabaseTask::Query(query, params, sender));

        let mut values = Vec::new();
        let mut error = false;
        while let Ok(item) = receiver.recv().await {
            match item {
                ItemStream::End => break,
                ItemStream::Error => { error = true; break },
                ItemStream::Value(v) => values.push(v)
            };
        }

        match error {
            false => Ok(values),
            true => Err(ResonateError::GenericError)
        }
    }
}

impl Database {
    pub fn new(root_dir: PathBuf) -> Database {

        let (task_sender, task_receiver) = unbounded();

        Database {
            _handle: spawn(move || database_thread(root_dir, task_receiver)),
            datalink: DataLink::new(task_sender)
        }
    }

    pub fn derive(&self) -> DataLink {
        self.datalink.clone()
    }
}

fn database_thread(root_dir: PathBuf, task_receiver: Receiver<DatabaseTask>) {

    let connection = match Connection::open(root_dir.join("data.db")) {
        Ok(connection) => connection,
        Err(_) => return
    };

    'mainloop: loop {
        let current_task = match task_receiver.recv_blocking() {
            Ok(task) => task,
            Err(_) => return
        };

        match current_task {
            DatabaseTask::Execute(query, params) => {
                if let Ok(mut statement) = connection.prepare(query) {
                    let _ = statement.execute(params.to_params());
                }
            },
            DatabaseTask::WaitExecute(query, params, sender) => {
                if let Ok(mut statement) = connection.prepare(query) {
                    let _ = statement.execute(params.to_params());
                    let _ = sender.send_blocking(());
                } else {
                    let _ = sender.send_blocking(());
                }
            },
            DatabaseTask::Insert(query, params, sender) => {
                if let Ok(mut statement) = connection.prepare(query) {
                    let _ = statement.execute(params.to_params());
                    let _ = sender.send_blocking(InsertMessage::Success(connection.last_insert_rowid() as usize));
                } else {
                    let _ = sender.send_blocking(InsertMessage::Error);
                }
            }
            DatabaseTask::Query(query, params, sender) => {
                let mut statement = match connection.prepare(query) {
                    Ok(statement) => statement,
                    Err(_) => {
                        let _ = sender.send_blocking(ItemStream::Error);
                        continue
                    }
                };

                let column_count = statement.column_count();
                let rows = match statement.query_map(params.to_params(), |row| {
                    let mut values = Vec::new();

                    'inner: for idx in 0..column_count {
                        let value = match row.get_ref(idx) {
                            Ok(value) => value,
                            Err(_) => continue 'inner
                        };

                        let value = match value {
                            ValueRef::Null => DatabaseParam::Null,
                            ValueRef::Integer(i) => DatabaseParam::Usize(i as usize),
                            ValueRef::Real(f) => DatabaseParam::F64(f),
                            ValueRef::Text(s) => DatabaseParam::String(String::from_utf8_lossy(s).into_owned()),
                            ValueRef::Blob(_) => DatabaseParam::Null,
                        };

                        values.push(value);
                    }

                    if column_count == values.len() { Ok(values) }
                    else { Err(rusqlite::Error::QueryReturnedNoRows) }
                }) {
                    Ok(rows) => rows.filter_map(|x| x.ok()).collect::<Vec<Vec<DatabaseParam>>>(),
                    Err(_) => {
                        let _ = sender.send_blocking(ItemStream::Error);
                        continue 'mainloop
                    }
                };

                for row in rows {
                    let _ = sender.send_blocking(ItemStream::Value(row));
                }
                let _ = sender.send_blocking(ItemStream::End);
            }
        }
    }
}
