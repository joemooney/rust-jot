use dotenv::dotenv;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

pub type Db = Pool<Postgres>;

pub async fn init_db() -> Result<Db, sqlx::Error> {
	dotenv().ok();
	let pg_host: String = std::env::var("PG_HOST").unwrap_or(String::from("localhost"));
	let pg_root_db: String = std::env::var("POSTGRES_DB").unwrap_or(String::from("postgres"));
	let pg_root_user: String = std::env::var("POSTGRES_USER").unwrap_or(String::from("postgres"));
	let pg_root_pwd: String = std::env::var("POSTGRES_PASSWORD").unwrap_or(String::from("postgres"));
	// app db
	let pg_app_db: String = std::env::var("PG_APP_DB").unwrap_or(String::from("app_db"));
	let pg_app_user: String = std::env::var("PG_APP_USER").unwrap_or(String::from("app_user"));
	let pg_app_pwd: String = std::env::var("PG_APP_PWD").unwrap_or(String::from("app_pwd_to_change"));
	let pg_app_max_con: u32 = 5;
	// sql files
	let sql_dir: String = std::env::var("SQL_DIR").unwrap_or(String::from("sql/"));
	let sql_recreate: String = std::env::var("SQL_RECREATE").unwrap_or(String::from("sql/00-recreate-db.sql"));

	println!("pg_root_db: {pg_root_db} pg_root_user: {pg_root_user} pg_root_pwd: {pg_root_pwd}");

	// -- Create the db with PG_ROOT (dev only)
	{
		let root_db = new_db_pool(&pg_host, &pg_root_db, &pg_root_user, &pg_root_pwd, 1).await?;
		pexec(&root_db, &sql_recreate).await?;
	}

	// -- Run the app sql files
	let app_db = new_db_pool(&pg_host, &pg_app_db, &pg_app_user, &pg_app_pwd, 1).await?;
	let mut paths: Vec<PathBuf> = fs::read_dir(sql_dir)?
		.into_iter()
		.filter_map(|e| e.ok().map(|e| e.path()))
		.collect();
	paths.sort();
	// execute each file
	for path in paths {
		if let Some(path) = path.to_str() {
			// only .sql and not the recreate
			if path.ends_with(".sql") && path != sql_recreate {
				pexec(&app_db, &path).await?;
			}
		}
	}

	// returning the app db
	new_db_pool(&pg_host, &pg_app_db, &pg_app_user, &pg_app_pwd, pg_app_max_con).await
}

async fn pexec(db: &Db, file: &str) -> Result<(), sqlx::Error> {
	// Read the file
	let content = fs::read_to_string(file).map_err(|ex| {
		println!("ERROR reading {} (cause: {:?})", file, ex);
		ex
	})?;

	// TODO: Make the split more sql proof
	let sqls: Vec<&str> = content.split(";").collect();

	for sql in sqls {
		match sqlx::query(&sql).execute(db).await {
			Ok(_) => (),
			Err(ex) => println!("WARNING - pexec - Sql file '{}' FAILED cause: {}", file, ex),
		}
	}

	Ok(())
}

async fn new_db_pool(host: &str, db: &str, user: &str, pwd: &str, max_con: u32) -> Result<Db, sqlx::Error> {
	let con_string = format!("postgres://{}:{}@{}/{}", user, pwd, host, db);
	PgPoolOptions::new()
		.max_connections(max_con)
		.acquire_timeout(Duration::from_millis(500)) // Needs to find replacement
		.connect(&con_string)
		.await
}

// region:    Test
#[cfg(test)]
#[path = "../_tests/model_db.rs"]
mod tests;
// endregion: Test
