use {
    crate::error::IngesterError,
    async_trait::async_trait,
    sea_orm::{DatabaseConnection, SqlxPostgresConnector},
    sqlx::{Pool, Postgres},
    std::fmt::Display,
    tokio::runtime::{Builder, Runtime},
    tokio::sync::mpsc::{self, UnboundedSender},
};

#[async_trait]
pub trait BgTask: Send + Sync + Display {
    async fn task(&self, db: &DatabaseConnection) -> Result<(), IngesterError>;
}

pub struct TaskManager {
    runtime: Runtime,
    producer: UnboundedSender<Box<dyn BgTask>>,
}

impl TaskManager {
    pub fn new(name: String, pool: Pool<Postgres>) -> Result<Self, IngesterError> {
        let runtime = Builder::new_multi_thread()
            .enable_all()
            .thread_name(name)
            .build()
            .map_err(|err| {
                IngesterError::TaskManagerError(format!(
                    "Could not create tokio runtime: {:?}",
                    err
                ))
            })?;

        let (producer, mut receiver) = mpsc::unbounded_channel::<Box<dyn BgTask>>();
        let db = SqlxPostgresConnector::from_sqlx_postgres_pool(pool);
        runtime.spawn(async move {
            while let Some(data) = receiver.recv().await {
                let task_res = data.task(&db).await;
                match task_res {
                    Ok(_) => println!("{} completed", data),
                    Err(e) => println!("{} errored with {:?}", data, e),
                }
            }
        });
        let tm = TaskManager { runtime, producer };
        Ok(tm)
    }

    pub fn get_sender(&self) -> UnboundedSender<Box<dyn BgTask>> {
        self.producer.clone()
    }
}
