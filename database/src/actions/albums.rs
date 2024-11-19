use std::collections::HashSet;
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use sea_orm::prelude::*;

use crate::actions::collection::CollectionQuery;
use crate::actions::utils::create_count_by_first_letter;
use crate::connection::MainDbConnection;
use crate::entities::{albums, media_file_albums, prelude};
use crate::{get_all_ids, get_by_id, get_by_ids, get_first_n, get_groups, collection_query};

use super::utils::CountByFirstLetter;

impl CountByFirstLetter for albums::Entity {
    fn group_column() -> Self::Column {
        albums::Column::Group
    }

    fn id_column() -> Self::Column {
        albums::Column::Id
    }
}

get_groups!(get_albums_groups, albums, media_file_albums, AlbumId);
get_all_ids!(get_media_file_ids_of_album, media_file_albums, AlbumId);
get_by_ids!(get_albums_by_ids, albums);
get_by_id!(get_album_by_id, albums);
get_first_n!(list_albums, albums);

collection_query!(
    albums::Model,
    prelude::Albums,
    0,
    "album",
    "lib::album",
    get_albums_groups,
    get_albums_by_ids,
    list_albums
);
