use metadata::describe::FileDescription;
use sea_orm::entity::prelude::*;
use sea_orm::{
    ColumnTrait, EntityTrait, FromQueryResult, Order, QueryFilter, QuerySelect, QueryTrait,
};
use std::path::Path;

use migration::{Func, SimpleExpr};

use crate::entities::{media_file_albums, media_file_artists, media_file_playlists, media_files};
use crate::{get_by_id, get_by_ids};

get_by_ids!(get_files_by_ids, media_files);
get_by_id!(get_file_by_id, media_files);

pub async fn get_random_files(
    db: &DatabaseConnection,
    n: usize,
) -> Result<Vec<media_files::Model>, sea_orm::DbErr> {
    let mut query: sea_orm::sea_query::SelectStatement =
        media_files::Entity::find().as_query().to_owned();
    let select = query
        .order_by_expr(SimpleExpr::FunctionCall(Func::random()), Order::Asc)
        .limit(n as u64);
    let statement = db.get_database_backend().build(select);

    let files = media_files::Model::find_by_statement(statement)
        .all(db)
        .await?;

    Ok(files)
}

pub async fn get_file_by_path(
    db: &DatabaseConnection,
    relative_path: &Path,
) -> Result<Option<media_files::Model>, sea_orm::DbErr> {
    let directory = relative_path
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .to_str()
        .unwrap_or("")
        .to_string();
    let file_name = relative_path
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new(""))
        .to_str()
        .unwrap_or("")
        .to_string();

    let file = media_files::Entity::find()
        .filter(media_files::Column::Directory.eq(directory))
        .filter(media_files::Column::FileName.eq(file_name))
        .one(db)
        .await?;

    Ok(file)
}

pub async fn get_file_id_from_path(
    db: &DatabaseConnection,
    root_path: &Path,
    file_path: &Path,
) -> Result<i32, String> {
    // Check if the file exists as an absolute path
    let absolute_path = if file_path.is_absolute() {
        file_path.to_path_buf()
    } else {
        root_path.join(file_path)
    };

    if !absolute_path.exists() {
        return Err(format!("File does not exist: {:?}", absolute_path));
    }

    let relative_path = match absolute_path.strip_prefix(root_path) {
        Ok(path) => path,
        Err(_) => {
            return Err(format!(
                "File is not within the specified library path: {:?}",
                absolute_path
            ));
        }
    };

    let file_info = match get_file_by_path(db, relative_path).await {
        Ok(Some(file_info)) => file_info,
        Ok(_none) => {
            return Err(format!("File is not in the database: {:?}", relative_path));
        }
        Err(e) => {
            return Err(format!("Failed to query the database: {}", e));
        }
    };

    Ok(file_info.id)
}

pub async fn get_media_files(
    db: &DatabaseConnection,
    cursor: usize,
    page_size: usize,
) -> Result<Vec<media_files::Model>, sea_orm::DbErr> {
    media_files::Entity::find()
        .cursor_by(media_files::Column::Id)
        .after(cursor as i32)
        .first(page_size as u64)
        .all(db)
        .await
}

pub async fn get_file_ids_by_descriptions(
    db: &DatabaseConnection,
    descriptions: &[Option<FileDescription>],
) -> Result<Vec<i32>, DbErr> {
    if descriptions.is_empty() {
        return Ok(vec![]);
    }

    let mut conditions = sea_orm::Condition::any();

    for description in descriptions {
        match description {
            Some(x) => {
                conditions = conditions.add(
                    media_files::Column::Directory
                        .eq(x.directory.clone())
                        .and(media_files::Column::FileName.eq(x.file_name.clone())),
                );
            }
            _none => {}
        }
    }

    let file_entries = media_files::Entity::find()
        .filter(conditions)
        .all(db)
        .await?;

    let file_ids = file_entries.into_iter().map(|entry| entry.id).collect();

    Ok(file_ids)
}

pub async fn get_duration_by_file_id(
    db: &DatabaseConnection,
    file_id: i32,
) -> Result<f64, sea_orm::DbErr> {
    let analysis_entry: Option<media_files::Model> = media_files::Entity::find()
        .filter(media_files::Column::Id.eq(file_id))
        .one(db)
        .await?;

    if let Some(entry) = analysis_entry {
        Ok(entry.duration)
    } else {
        Err(sea_orm::DbErr::RecordNotFound(
            "Analysis record not found".to_string(),
        ))
    }
}

pub async fn compound_query_media_files(
    db: &DatabaseConnection,
    artist_ids: Option<Vec<i32>>,
    album_ids: Option<Vec<i32>>,
    playlist_ids: Option<Vec<i32>>,
    directories: Option<Vec<&str>>,
    cursor: usize,
    page_size: usize,
) -> Result<Vec<media_files::Model>, sea_orm::DbErr> {
    use sea_orm::sea_query::Condition;
    use sea_orm::sea_query::Expr;

    // Base query for media_files
    let mut query = media_files::Entity::find();

    // Filter by artist_ids if provided
    if let Some(artist_ids) = artist_ids {
        let artist_subquery = media_file_artists::Entity::find()
            .select_only()
            .filter(media_file_artists::Column::ArtistId.is_in(artist_ids))
            .column(media_file_artists::Column::MediaFileId)
            .into_query();

        query = query.filter(
            Condition::all().add(Expr::col(media_files::Column::Id).in_subquery(artist_subquery)),
        );
    }

    // Filter by album_ids if provided
    if let Some(album_ids) = album_ids {
        let album_subquery = media_file_albums::Entity::find()
            .select_only()
            .filter(media_file_albums::Column::AlbumId.is_in(album_ids))
            .column(media_file_albums::Column::MediaFileId)
            .into_query();

        query = query.filter(
            Condition::all().add(Expr::col(media_files::Column::Id).in_subquery(album_subquery)),
        );
    }

    // Filter by playlist_ids if provided
    if let Some(playlist_ids) = playlist_ids {
        let playlist_subquery = media_file_playlists::Entity::find()
            .select_only()
            .filter(media_file_playlists::Column::PlaylistId.is_in(playlist_ids))
            .column(media_file_playlists::Column::MediaFileId)
            .into_query();

        query = query.filter(
            Condition::all().add(Expr::col(media_files::Column::Id).in_subquery(playlist_subquery)),
        );
    }

    // Filter by directories if provided
    if let Some(directories) = directories {
        let mut dir_conditions = Condition::any();
        for dir in directories {
            let dir = dir.strip_prefix('/').unwrap_or(dir);

            dir_conditions = dir_conditions.add(
                Expr::col(media_files::Column::Directory)
                    .eq(dir)
                    .or(Expr::col(media_files::Column::Directory).like(format!("{}/%", dir))),
            );
        }
        query = query.filter(dir_conditions);
    }

    // Use cursor pagination
    let mut cursor_by_id = query.cursor_by(media_files::Column::Id);

    // Retrieve the specified number of rows
    let media_files = cursor_by_id
        .after(cursor as i32)
        .first(page_size as u64)
        .all(db)
        .await?;

    Ok(media_files)
}
