pub mod utils;

use utils::setup_functions::*;

#[cfg(test)]
mod tests {
    use sea_orm::{
        entity::prelude::*, entity::*, tests_cfg::*,
        DatabaseBackend, MockDatabase, MockExecResult, Transaction,
    };



    
    #[async_std::test]
    async fn test_get_asset() -> Result<(), DbErr> {

        ingester_setup().await?;

        let db = MockDatabaseConnector::connect().await?;

       

        // Create MockDatabase with mock execution result
        // let db = MockDatabase::new(DatabaseBackend::Postgres)
        //     .append_query_results(vec![
        //         vec![cake::Model {
        //             id: 15,
        //             name: "Apple Pie".to_owned(),
        //         }],
        //         vec![cake::Model {
        //             id: 16,
        //             name: "Apple Pie".to_owned(),
        //         }],
        //     ])
        //     .append_exec_results(vec![
        //         MockExecResult {
        //             last_insert_id: 15,
        //             rows_affected: 1,
        //         },
        //         MockExecResult {
        //             last_insert_id: 16,
        //             rows_affected: 1,
        //         },
        //     ])
        //     .into_connection();

    

    

        Ok(())
    }
}