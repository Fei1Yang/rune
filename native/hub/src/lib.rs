mod album;
mod artist;
mod common;
mod connection;
mod cover_art;
mod directory;
mod library_home;
mod library_manage;
mod media_file;
mod messages;
mod playback;
mod player;
mod playlist;
mod recommend;
mod search;

use log::{debug, info};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::filter::EnvFilter;

pub use tokio;

use ::database::connection::connect_main_db;
use ::database::connection::connect_recommendation_db;
use ::database::connection::connect_search_db;
use ::playback::player::Player;

use crate::album::*;
use crate::artist::*;
use crate::connection::*;
use crate::cover_art::*;
use crate::directory::*;
use crate::library_home::*;
use crate::library_manage::*;
use crate::media_file::*;
use crate::playback::*;
use crate::player::initialize_player;
use crate::playlist::*;
use crate::recommend::*;
use crate::search::*;

use messages::album::*;
use messages::artist::*;
use messages::cover_art::*;
use messages::directory::*;
use messages::library_home::*;
use messages::library_manage::*;
use messages::media_file::*;
use messages::playback::*;
use messages::playlist::*;
use messages::recommend::*;
use messages::search::*;

macro_rules! select_signal {
    ($cancel_token:expr, $( $type:ty => ($($arg:ident),*) ),* $(,)? ) => {
        paste::paste! {
            $(
                let mut [<receiver_ $type:snake>] = <$type>::get_dart_signal_receiver().unwrap();
            )*

            loop {
                if $cancel_token.is_cancelled() {
                    info!("Cancellation requested. Exiting main loop.");
                    break;
                }

                tokio::select! {
                    $(
                        dart_signal = [<receiver_ $type:snake>].recv() => {
                            if let Some(dart_signal) = dart_signal {
                                debug!("Processing signal: {}", stringify!($type));
                                let handler_fn = [<$type:snake>];
                                let _ = handler_fn($($arg.clone()),*, dart_signal).await;
                            }
                        }
                    )*
                    else => continue,
                }
            }
        }
    };
}

rinf::write_interface!();

async fn player_loop(path: String) {
    // Ensure that the path is set before calling fetch_media_files

    info!("Media Library Received, initialize other receivers");

    tokio::spawn(async {
        // Move the path into the async block
        info!("Initializing database");
        let main_db = Arc::new(connect_main_db(&path).await.unwrap());
        let recommend_db = Arc::new(connect_recommendation_db(&path).unwrap());
        let search_db = Arc::new(Mutex::new(connect_search_db(&path).unwrap()));
        let lib_path = Arc::new(path);

        // Create a cancellation token
        let cancel_token = CancellationToken::new();

        info!("Initializing player");
        let player = Player::new(Some(cancel_token.clone()));
        let player = Arc::new(Mutex::new(player));

        let cancel_token = Arc::new(cancel_token);

        info!("Initializing Player events");
        tokio::spawn(initialize_player(main_db.clone(), player.clone()));

        info!("Initializing UI events");

        select_signal!(
            cancel_token,

            CloseLibraryRequest => (lib_path, cancel_token),
            ScanAudioLibraryRequest => (main_db, search_db, cancel_token),
            AnalyseAudioLibraryRequest => (main_db, recommend_db, cancel_token),

            PlayFileRequest => (main_db, lib_path, player),
            PlayRequest => (player),
            PauseRequest => (player),
            NextRequest => (player),
            PreviousRequest => (player),
            SwitchRequest => (player),
            SeekRequest => (player),
            RemoveRequest => (player),

            RecommendAndPlayRequest => (main_db, recommend_db, lib_path, player),
            RecommendAndPlayMixRequest => (main_db, recommend_db, lib_path, player),
            
            FetchMediaFilesRequest => (main_db, lib_path),
            FetchParsedMediaFileRequest => (main_db, lib_path),
            CompoundQueryMediaFilesRequest => (main_db, lib_path),

            StartPlayingCollectionRequest => (main_db, lib_path, player),
            AddToQueueCollectionRequest => (main_db, lib_path, player),
            FetchMediaFileByIdsRequest => (main_db, lib_path),
            StartRoamingCollectionRequest => (main_db, recommend_db, lib_path, player),

            GetCoverArtByFileIdRequest => (main_db, lib_path),
            GetCoverArtByCoverArtIdRequest => (main_db),
            GetRandomCoverArtIdsRequest => (main_db),

            FetchArtistsGroupSummaryRequest => (main_db),
            FetchArtistsGroupsRequest => (main_db),
            FetchArtistsByIdsRequest => (main_db),

            FetchAlbumsGroupSummaryRequest => (main_db),
            FetchAlbumsGroupsRequest => (main_db),
            FetchAlbumsByIdsRequest => (main_db),

            FetchPlaylistsGroupSummaryRequest => (main_db),
            FetchPlaylistsGroupsRequest => (main_db),
            FetchPlaylistsByIdsRequest => (main_db),
            FetchAllPlaylistsRequest => (main_db),
            MovePlaylistItemRequest => (player),
            CreatePlaylistRequest => (main_db, search_db),
            UpdatePlaylistRequest => (main_db, search_db),
            CheckItemsInPlaylistRequest => (main_db),
            AddItemToPlaylistRequest => (main_db),
            AddMediaFileToPlaylistRequest => (main_db),
            ReorderPlaylistItemPositionRequest => (main_db),
            GetUniquePlaylistGroupsRequest => (main_db),
            GetPlaylistByIdRequest => (main_db),

            FetchLibrarySummaryRequest => (main_db),
            SearchForRequest => (search_db),

            FetchDirectoryTreeRequest => (main_db),
        );
    });
}

async fn main() {
    let filter = EnvFilter::new(
        "symphonia_format_ogg=off,symphonia_core=off,symphonia_bundle_mp3::demuxer=off,sea_orm_migration::migrator=off,info",
    );

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_test_writer()
        .init();

    // Start receiving the media library path
    let _ = receive_media_library_path(player_loop).await;
}
