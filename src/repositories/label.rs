use axum::async_trait;
use serde::{Deserialize,Serialize};
use sqlx::PgPool;

use super::RepositoryError;

#[async_trait]
pub trait LabelRepository: Clone + std::marker::Send + std::marker::Sync + 'static {
    async fn create(&self, name: String) -> anyhow::Result<Label>;
    async fn all(&self) -> anyhow::Result<Vec<Label>>;
    async fn delete(&self, id:i32) -> anyhow::Result<()>;
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct Label {
    pub id: i32,
    pub name: String,
}

// #[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
// pub struct UpdateLabel{
//     id: i32,
//     name: String,    
// }

#[derive(Debug, Clone)]
pub struct LabelRepositoryForDb{
    pool:PgPool,
}

impl LabelRepositoryForDb {
    pub fn new(pool: PgPool) -> Self{
        Self { pool }
    }
}

#[async_trait]
impl LabelRepository for LabelRepositoryForDb {
    async fn create(&self, name: String) -> anyhow::Result<Label>{
        let optional_label = sqlx::query_as::<_, Label>(
            r#"SELECT * FROM labels WHERE name = $1"#
        )
        .bind(name.clone())
        .fetch_optional(&self.pool)
        .await?;

        if let Some(label) = optional_label {
            return Err(RepositoryError::Duplicate(label.id).into());
        }

        let label = sqlx::query_as::<_,Label>(
            r#"INSERT INTO labels(name) VALUES ($1) returning *"#
        )
        .bind(name.clone())
        .fetch_one(&self.pool)
        .await?;
        Ok(label)
        
    }


    async fn all(&self) -> anyhow::Result<Vec<Label>>{
        let labels = sqlx::query_as::<_, Label>(
            r#"SELECT * FROM labels ORDER BY labels.id asc"#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(labels)
    }


    async fn delete(&self, id:i32) -> anyhow::Result<()>{
        sqlx::query(
            r#"DELETE FROM labels WHERE id=$1"#
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => RepositoryError::NotFound(id),
            _ => RepositoryError::Unexpected(e.to_string()),
        })?;

        Ok(())
    }
}

#[cfg(test)]
#[cfg(feature="database-test")]
mod test {
    use super::*;
    use dotenv::dotenv;
    use sqlx::PgPool;
    use std::env;

    #[tokio::test]
    async fn crud_scenario(){

        dotenv().ok();
        let database_url = &env::var("DATABASE_URL").expect("undefined [DATABASE_URL]");
        let pool = PgPool::connect(database_url)
            .await
            .expect(&format!("fail connect database, url is [{}]", database_url));
        
        let repository = LabelRepositoryForDb::new(pool);
        let label_text = "test_label";

        // create
        let label = repository
            .create(label_text.to_string())
            .await
            .expect("[create] returned Err");

        assert_eq!(label.name, label_text);
        println!("label.id [{}]", label.id);

        // all
        let labels = repository
            .all()
            .await
            .expect("[all] returned Err");

        let label = labels.last().unwrap();

        assert_eq!(label.name, label_text);

        // delete
        repository
            .delete(label.id)
            .await
            .expect("[delete] returned Err");

    }


}



// test without DB connection
#[cfg(test)]
pub mod test_utils{
    // use anyhow::Context;
    use axum::async_trait;
    use std::{
        collections::HashMap,
        sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard },
    };
    use super::*;

    impl Label {
        pub fn new(id:i32, name:String) -> Self{
            Self { 
                id, 
                name, 
            }
        }
    }

    type LabelDatas = HashMap<i32, Label>;

    #[derive(Debug, Clone)]
    pub struct LabelRepositoryForMemory{
        store: Arc<RwLock<LabelDatas>>,
    }
    
    impl LabelRepositoryForMemory {
        pub fn new() -> Self {
            LabelRepositoryForMemory { 
                store: Arc::default(), 
            }
        }
    
        fn write_store_ref(&self) -> RwLockWriteGuard<LabelDatas>{
            self.store.write().unwrap()
        }
    
        fn read_store_ref(&self) -> RwLockReadGuard<LabelDatas>{
            self.store.read().unwrap()
        }
    
    }
    
    #[async_trait]
    impl LabelRepository for LabelRepositoryForMemory {

        async fn create(&self, name: String) -> anyhow::Result<Label> {
            let mut store = self.write_store_ref();
            let id  = (store.len()+1) as i32;
            let label = Label::new(id, name.clone());
            store.insert(id, label.clone());
            
            Ok(label)
        }
    
    
        async fn all(&self) -> anyhow::Result<Vec<Label>> {
            let store = self.read_store_ref();
            Ok(Vec::from_iter(store.values().map(|label| label.clone())))
        }
    
    
        async fn delete(&self, id:i32) -> anyhow::Result<()> {
            let mut store = self.write_store_ref();
            store.remove(&id).ok_or(RepositoryError::NotFound(id))?;
            Ok(())
        }
    }
    
    mod test {
        use super::*;
        
        #[tokio::test]
        async fn label_crud_scenario(){

            let name = "label name".to_string();
            let id = 1;
            let expected = Label::new(id, name.clone());


            // create
            let repository = LabelRepositoryForMemory::new();
            let label = repository
                .create(name)
                .await
                .expect("failed create label");
            assert_eq!(expected, label);

            // all
            let label = repository.all().await.expect("failed get all label");
            assert_eq!(vec![expected], label);

            // delete
            let res = repository.delete(id).await;
            assert!(res.is_ok())

        }
    }

}