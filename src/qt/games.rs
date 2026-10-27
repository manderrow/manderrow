//! The `Games` QML singleton and the `Game` type, through which the frontend reads the games that
//! Manderrow supports.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qurl.h");
        type QUrl = cxx_qt_lib::QUrl;
        include!(<QtCore/QObject>);
        include!("cxx-qt-lib/qlist.h");
        type QObjectList = crate::qt::QObjectList;
    }

    /// What the game selection screen sorts games by, after their relevance to the search.
    #[qenum(Games)]
    enum SortColumn {
        ModDownloads,
        Popularity,
        Name,
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        /// A game that Manderrow supports. The games are created once, by [`Games`], which owns
        /// them, so they can be compared by identity.
        #[qobject]
        #[qml_element]
        #[qml_uncreatable]
        #[qproperty(QString, id, READ, CONSTANT)]
        #[qproperty(QString, name, READ, CONSTANT)]
        /// The id of the game's Thunderstore community.
        #[qproperty(QString, thunderstore_id, READ, CONSTANT)]
        #[qproperty(QUrl, cover_url, READ, CONSTANT)]
        #[qproperty(QUrl, hero_url, READ, CONSTANT)]
        type Game = super::GameRust;

        /// Every [`Game`], in the order of `games.json`.
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(QObjectList, all, READ, CONSTANT)]
        type Games = super::GamesRust;

        /// Returns the game with an id, or null if there is none.
        #[qinvokable]
        fn by_id(self: &Games, id: &QString) -> *mut Game;

        /// Returns the games whose names match a query, the most relevant first, and then by a
        /// column. An empty query matches every game.
        #[qinvokable]
        fn search(self: &Games, query: &QString, sort: SortColumn) -> QObjectList;
    }

    #[namespace = "rust::cxxqtlib1"]
    unsafe extern "C++" {
        include!("cxx-qt-lib/common.h");

        #[rust_name = "new_game"]
        fn make_unique() -> UniquePtr<Game>;
    }

    impl cxx_qt::Initialize for Games {}
}

use core::pin::Pin;
use std::collections::HashMap;

use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QUrl};

use super::child::Child;
use super::QObjectList;
use crate::games::{self, SortColumn};
use crate::util::search::SortOption;

#[derive(Default)]
pub struct GameRust {
    id: QString,
    name: QString,
    thunderstore_id: QString,
    cover_url: QUrl,
    hero_url: QUrl,
}

#[derive(Default)]
pub struct GamesRust {
    /// The games, in the order of [`games::games`].
    games: Vec<Child<qobject::Game>>,
    /// The indices of the games, by id.
    by_id: HashMap<&'static str, usize>,
    all: QObjectList,
}

impl cxx_qt::Initialize for qobject::Games {
    fn initialize(mut self: Pin<&mut Self>) {
        let data = match games::games() {
            Ok(data) => data,
            Err(e) => {
                slog_scope::error!("Unable to load the games: {e:#}");
                return;
            }
        };
        let mut objects = Vec::with_capacity(data.len());
        for game in data {
            let mut object = qobject::new_game();
            {
                let mut rust = object.pin_mut().rust_mut();
                rust.id = QString::from(game.id);
                rust.name = QString::from(&*game.name);
                rust.thunderstore_id = QString::from(game.thunderstore_id);
                rust.cover_url = QUrl::from(&*format!(
                    "qrc:/assets/img/game_covers/{}.webp",
                    game.thunderstore_id
                ));
                rust.hero_url =
                    QUrl::from(&*format!("qrc:/assets/img/game_heros/{}.webp", game.id));
            }
            objects.push(Child::adopt(self.as_mut(), object));
        }

        let mut rust = self.rust_mut();
        rust.by_id = data.iter().enumerate().map(|(i, game)| (game.id, i)).collect();
        rust.all = objects.iter().map(Child::as_qobject).collect();
        rust.games = objects;
    }
}

impl qobject::Games {
    fn by_id(&self, id: &QString) -> *mut qobject::Game {
        self.by_id
            .get(&*id.to_string())
            .map_or(std::ptr::null_mut(), |&i| self.games[i].as_ptr())
    }

    fn search(&self, query: &QString, sort: qobject::SortColumn) -> QObjectList {
        let column = match sort {
            qobject::SortColumn::ModDownloads => SortColumn::ModDownloads,
            qobject::SortColumn::Popularity => SortColumn::Popularity,
            qobject::SortColumn::Name => SortColumn::Name,
            // QML can pass any number
            _ => SortColumn::ModDownloads,
        };
        let sort = [
            SortOption {
                column: SortColumn::Relevance,
                descending: true,
            },
            // names read from A to Z, and the others from the most to the least
            SortOption {
                column,
                descending: !matches!(column, SortColumn::Name),
            },
        ];
        match games::search(&query.to_string(), &sort) {
            Ok(indices) => indices
                .into_iter()
                .map(|i| self.games[i].as_qobject())
                .collect(),
            Err(e) => {
                slog_scope::error!("Unable to search the games: {e:#}");
                QObjectList::default()
            }
        }
    }
}
