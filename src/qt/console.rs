//! The `Console` QML singleton: the connections to the games the app has launched, or that have
//! connected to it, with their logs, and the problems they've reported.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!(<QtCore/QObject>);
        include!("cxx-qt-lib/qlist.h");
        type QObjectList = crate::qt::QObjectList;
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
        include!("cxx-qt-lib/qdatetime.h");
        type QDateTime = cxx_qt_lib::QDateTime;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;
        include!("cxx-qt-lib/qmap.h");
        type QVariantMap = crate::qt::QVariantMap;
        include!("cxx-qt-lib/qhash.h");
        type QHash_i32_QByteArray = cxx_qt_lib::QHash<cxx_qt_lib::QHashPair_i32_QByteArray>;
        include!("cxx-qt-lib/qmodelindex.h");
        type QModelIndex = cxx_qt_lib::QModelIndex;
        include!("manderrow/src/qt/call.cxxqt.h");
        type Call = crate::qt::call::Call;
    }

    unsafe extern "C++Qt" {
        include!(<QtCore/QAbstractListModel>);
        #[qobject]
        type QAbstractListModel;
    }

    #[qenum(Connection)]
    enum ConnectionStatus {
        /// Waiting for the game to connect.
        Connecting,
        Connected,
        Disconnected,
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_uncreatable]
        #[base = QAbstractListModel]
        /// How many lines it has.
        #[qproperty(i32, count, READ, NOTIFY)]
        type ConsoleLog = super::ConsoleLogRust;

        #[cxx_override]
        fn row_count(self: &ConsoleLog, parent: &QModelIndex) -> i32;
        #[cxx_override]
        fn data(self: &ConsoleLog, index: &QModelIndex, role: i32) -> QVariant;
        #[cxx_override]
        fn role_names(self: &ConsoleLog) -> QHash_i32_QByteArray;

        /// A line, as an object of its roles.
        #[qinvokable]
        fn get(self: &ConsoleLog, row: i32) -> QVariantMap;

        #[inherit]
        fn begin_insert_rows(
            self: Pin<&mut ConsoleLog>,
            parent: &QModelIndex,
            first: i32,
            last: i32,
        );
        #[inherit]
        fn end_insert_rows(self: Pin<&mut ConsoleLog>);

        #[qobject]
        #[qml_element]
        #[qml_uncreatable]
        #[qproperty(i32, id, READ, CONSTANT)]
        /// The profile it was launched from, or empty.
        #[qproperty(QString, profile_id, READ, CONSTANT)]
        #[qproperty(ConnectionStatus, status, READ, NOTIFY)]
        #[qproperty(QDateTime, created_time, READ, CONSTANT)]
        /// The lines of its log, a model with the roles `type`, `dataType`, `scope`, `message`,
        /// `filter` (the toggle that shows it), and `searchable`.
        #[qproperty(*mut ConsoleLog, log, READ, CONSTANT)]
        /// The distinct types and scopes of its log's lines, which size the log's columns.
        #[qproperty(QStringList, type_labels, READ, NOTIFY)]
        #[qproperty(QStringList, scopes, READ, NOTIFY)]
        type Connection = super::ConnectionRust;

        /// Launches a game for the connection, from a profile, or unmodded without one. An error
        /// is logged as well as returned.
        #[qinvokable]
        fn launch(
            self: &Connection,
            game: &QString,
            profile_id: &QString,
            modded: bool,
        ) -> *mut Call;

        /// Kills the game. An error is logged as well as returned.
        #[qinvokable]
        fn kill(self: &Connection) -> *mut Call;

        #[qobject]
        #[qml_element]
        #[qml_uncreatable]
        #[qproperty(QString, id, READ, CONSTANT)]
        /// Labels and a description by locale, or by the placeholder of the fix's translations.
        #[qproperty(QVariantMap, label, READ, CONSTANT)]
        #[qproperty(QVariantMap, confirm_label, READ, CONSTANT)]
        #[qproperty(QVariantMap, description, READ, CONSTANT)]
        type DoctorFix = super::DoctorFixRust;

        #[qobject]
        #[qml_element]
        #[qml_uncreatable]
        /// The key of its translations, under `doctor`.
        #[qproperty(QString, translation_key, READ, CONSTANT)]
        /// A message that takes the place of its translation, if it has one.
        #[qproperty(QString, message, READ, CONSTANT)]
        #[qproperty(QVariantMap, message_args, READ, CONSTANT)]
        /// Its `DoctorFix`es.
        #[qproperty(QObjectList, fixes, READ, CONSTANT)]
        type DoctorReport = super::DoctorReportRust;

        /// Tells the game which fix to make.
        #[qinvokable]
        fn choose(self: &DoctorReport, fix: &QString) -> *mut Call;

        #[qobject]
        #[qml_element]
        #[qml_singleton]
        /// Every `Connection`, by id.
        #[qproperty(QObjectList, connections, READ, NOTIFY)]
        /// The connection whose log is shown.
        #[qproperty(*mut Connection, focused, READ, WRITE, NOTIFY)]
        /// The `DoctorReport`s that haven't been dismissed.
        #[qproperty(QObjectList, doctor_reports, READ, NOTIFY)]
        type Console = super::ConsoleRust;

        /// Makes a connection for a game about to be launched from a profile, or without one.
        #[qinvokable]
        fn allocate(self: Pin<&mut Console>, profile_id: &QString) -> *mut Connection;

        #[qinvokable]
        fn dismiss_doctor_report(self: Pin<&mut Console>, index: i32);
    }

    impl cxx_qt::Threading for Console {}
    impl cxx_qt::Initialize for Console {}

    #[namespace = "rust::cxxqtlib1"]
    unsafe extern "C++" {
        include!("cxx-qt-lib/common.h");
        #[rust_name = "new_console_log"]
        fn make_unique() -> UniquePtr<ConsoleLog>;
        #[rust_name = "new_connection"]
        fn make_unique() -> UniquePtr<Connection>;
        #[rust_name = "new_doctor_fix"]
        fn make_unique() -> UniquePtr<DoctorFix>;
        #[rust_name = "new_doctor_report"]
        fn make_unique() -> UniquePtr<DoctorReport>;
    }
}

use core::pin::Pin;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use cxx_qt::casting::Upcast;
use cxx_qt::{CxxQtType, QObject, Threading};
use cxx_qt_lib::{
    QByteArray, QDateTime, QHash, QHashPair_i32_QByteArray, QModelIndex, QString, QStringList,
    QTimeZone, QVariant,
};
use uuid::Uuid;

use super::call::{self, Call};
use super::child::Child;
use super::outbox::Outbox;
use super::{QObjectList, QVariantMap};
use crate::ipc::{
    C2SMessage, ConnectionId, DoctorReport, Event, IpcState, LogLevel, OutputLine, S2CMessage,
    SafeOsString, StandardOutputChannel,
};
use crate::launching::LaunchTarget;
use crate::CommandError;
use qobject::{Connection, ConnectionStatus, Console, ConsoleLog, QAbstractListModel};

cxx_qt::impl_transitive_cast!(ConsoleLog, QAbstractListModel, QObject);

/// A line of a game's log.
struct Line {
    kind: String,
    data_type: String,
    scope: String,
    message: String,
    /// The toggle that shows it, if one does.
    filter: String,
    /// Whether the log's search looks in it.
    searchable: bool,
}

const ROLES: [&str; 6] = [
    "type",
    "dataType",
    "scope",
    "message",
    "filter",
    "searchable",
];
/// Qt::UserRole, after which a model's own roles are numbered
const USER_ROLE: i32 = 0x0100;

impl Line {
    fn role(&self, role: usize) -> QVariant {
        match role {
            0 => QVariant::from(&QString::from(&self.kind)),
            1 => QVariant::from(&QString::from(&self.data_type)),
            2 => QVariant::from(&QString::from(&self.scope)),
            3 => QVariant::from(&QString::from(&self.message)),
            4 => QVariant::from(&QString::from(&self.filter)),
            5 => QVariant::from(&self.searchable),
            _ => QVariant::default(),
        }
    }

    /// The line a message is shown as, if it's shown as one.
    fn of(msg: &C2SMessage) -> Option<Self> {
        let line = |kind: &str, data_type: &str, message: String| Self {
            kind: kind.to_owned(),
            data_type: data_type.to_owned(),
            scope: String::new(),
            message,
            filter: String::new(),
            searchable: false,
        };
        Some(match msg {
            C2SMessage::Output {
                channel,
                line: output,
            } => {
                let label = match channel {
                    StandardOutputChannel::Out => "STDOUT",
                    StandardOutputChannel::Err => "STDERR",
                };
                let (message, searchable) = match output {
                    OutputLine::Unicode(text) => (text.clone(), true),
                    OutputLine::Bytes(bytes) => (json(bytes), false),
                };
                Self {
                    filter: label.to_owned(),
                    searchable,
                    ..line(label, "", message)
                }
            }
            C2SMessage::Log {
                level,
                scope,
                message,
            } => {
                let level = level_name(*level);
                Self {
                    scope: format!("{scope}:"),
                    filter: level.to_owned(),
                    searchable: true,
                    ..line(level, level, message.clone())
                }
            }
            C2SMessage::Connect { .. } => line(
                "CONNECT",
                "CONNECT",
                "Game connected to Manderrow".to_owned(),
            ),
            C2SMessage::Start { command, args, env } => {
                let mut message = String::new();
                let mut env = env.iter().collect::<Vec<_>>();
                env.sort_by_key(|(key, _)| *key);
                for (key, value) in env {
                    let value = match value {
                        SafeOsString::Unicode(value) => json(value),
                        value => json(value),
                    };
                    message.push_str(&format!("{key}={value} "));
                }
                message.push_str(&os_string(command));
                message.push(' ');
                for arg in args {
                    message.push(' ');
                    message.push_str(&os_string(arg));
                }
                line("START", "", message)
            }
            C2SMessage::Started { pid } => line(
                "STARTED",
                "STARTED",
                format!("Game process started with id {pid}"),
            ),
            C2SMessage::Exit { code } => line(
                "EXIT",
                "",
                code.map_or_else(|| "Unknown exit code".to_owned(), |code| code.to_string()),
            ),
            C2SMessage::Crash { error } => line("CRASH", "CRASH", error.clone()),
            C2SMessage::DoctorReport(_) => return None,
        })
    }

    fn error(error: &CommandError) -> Self {
        let message = match error {
            CommandError::Aborted => "Aborted by the user".to_owned(),
            CommandError::Error { messages, .. } => messages.join("\n"),
        };
        Self {
            kind: "ERROR".to_owned(),
            data_type: "ERROR".to_owned(),
            scope: String::new(),
            message,
            filter: String::new(),
            searchable: false,
        }
    }
}

/// The name of a level, which is that of the toggle that shows its lines.
fn level_name(level: LogLevel) -> &'static str {
    match level {
        LogLevel::Critical => "CRITICAL",
        LogLevel::Error => "ERROR",
        LogLevel::Warning => "WARN",
        LogLevel::Info => "INFO",
        LogLevel::Debug => "DEBUG",
        LogLevel::Trace => "TRACE",
    }
}

fn json(value: &(impl serde::Serialize + ?Sized)) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

fn os_string(s: &SafeOsString) -> String {
    match s {
        SafeOsString::Unicode(s) => json(s),
        SafeOsString::NonUnicodeBytes(bytes) => json(bytes),
        SafeOsString::NonUnicodeWide(wide) => json(wide),
    }
}

#[derive(Default)]
pub struct ConsoleLogRust {
    count: i32,
    lines: Vec<Line>,
}

impl ConsoleLog {
    fn row_count(&self, parent: &QModelIndex) -> i32 {
        // a list has rows only at its root
        if parent.is_valid() {
            0
        } else {
            self.count
        }
    }

    fn data(&self, index: &QModelIndex, role: i32) -> QVariant {
        let line = usize::try_from(index.row())
            .ok()
            .and_then(|row| self.lines.get(row));
        let role = usize::try_from(role - USER_ROLE).ok();
        match (line, role) {
            (Some(line), Some(role)) => line.role(role),
            _ => QVariant::default(),
        }
    }

    fn role_names(&self) -> QHash<QHashPair_i32_QByteArray> {
        let mut roles = QHash::<QHashPair_i32_QByteArray>::default();
        for (i, name) in ROLES.iter().enumerate() {
            roles.insert(USER_ROLE + i as i32, QByteArray::from(*name));
        }
        roles
    }

    fn get(&self, row: i32) -> QVariantMap {
        let line = usize::try_from(row)
            .ok()
            .and_then(|row| self.lines.get(row));
        line.into_iter()
            .flat_map(|line| (0..ROLES.len()).map(|i| (QString::from(ROLES[i]), line.role(i))))
            .collect()
    }

    /// Adds lines to the end of the log together, which a view lays out far faster than one by
    /// one, as a game can log thousands of lines at once.
    fn append(mut self: Pin<&mut Self>, lines: Vec<Line>) {
        if lines.is_empty() {
            return;
        }
        let first = self.count;
        let last = first + lines.len() as i32 - 1;
        self.as_mut()
            .begin_insert_rows(&QModelIndex::default(), first, last);
        let mut rust = self.as_mut().rust_mut();
        rust.lines.extend(lines);
        rust.count = last + 1;
        self.as_mut().end_insert_rows();
        self.count_changed();
    }
}

pub struct ConnectionRust {
    id: i32,
    profile_id: QString,
    status: ConnectionStatus,
    created_time: QDateTime,
    log: *mut ConsoleLog,
    type_labels: QStringList,
    scopes: QStringList,
    log_object: Option<Child<ConsoleLog>>,
    console: Option<Arc<Outbox<Console>>>,
}

impl Default for ConnectionRust {
    fn default() -> Self {
        Self {
            id: -1,
            profile_id: QString::default(),
            status: ConnectionStatus::Connecting,
            created_time: QDateTime::from_msecs_since_epoch(
                chrono::Utc::now().timestamp_millis(),
                &QTimeZone::utc(),
            ),
            log: std::ptr::null_mut(),
            type_labels: QStringList::default(),
            scopes: QStringList::default(),
            log_object: None,
            console: None,
        }
    }
}

impl Connection {
    fn conn_id(&self) -> ConnectionId {
        ConnectionId::from_qml(self.id)
    }

    fn set_status(mut self: Pin<&mut Self>, status: ConnectionStatus) {
        if self.status != status {
            self.as_mut().rust_mut().status = status;
            self.status_changed();
        }
    }

    fn append(mut self: Pin<&mut Self>, lines: Vec<Line>) {
        let mut labels_changed = false;
        let mut scopes_changed = false;
        {
            let mut rust = self.as_mut().rust_mut();
            for line in &lines {
                let kind = QString::from(&line.kind);
                if !cxx_qt_lib::QList::contains(&rust.type_labels, &kind) {
                    rust.type_labels.append(kind);
                    labels_changed = true;
                }
                if !line.scope.is_empty() {
                    let scope = QString::from(&line.scope);
                    if !cxx_qt_lib::QList::contains(&rust.scopes, &scope) {
                        rust.scopes.append(scope);
                        scopes_changed = true;
                    }
                }
            }
        }
        if labels_changed {
            self.as_mut().type_labels_changed();
        }
        if scopes_changed {
            self.as_mut().scopes_changed();
        }
        if let Some(log) = &mut self.as_mut().rust_mut().log_object {
            log.get_mut().append(lines);
        }
    }

    fn launch(&self, game: &QString, profile_id: &QString, modded: bool) -> *mut Call {
        let (game, profile_id) = (game.to_string(), profile_id.to_string());
        let conn_id = self.conn_id();
        self.logging_errors(async move {
            let app = super::app();
            let target = match profile_id.as_str() {
                "" => LaunchTarget::Vanilla(&game),
                id => LaunchTarget::Profile(Uuid::try_parse(id).map_err(anyhow::Error::from)?),
            };
            crate::launching::launch_profile(
                app.clone(),
                &app.state::<IpcState>(),
                target,
                modded,
                conn_id,
            )
            .await
            .map_err(CommandError::from)
        })
    }

    fn kill(&self) -> *mut Call {
        let conn_id = self.conn_id();
        self.logging_errors(async move {
            let conn = super::app()
                .state::<IpcState>()
                .get_conn(conn_id)
                .ok_or_else(|| anyhow::anyhow!("No such connection: {conn_id}"))?;
            conn.kill_process(&slog_scope::logger())
                .map_err(anyhow::Error::from)?;
            Ok(())
        })
    }

    /// Starts a call to do something with the connection's game, whose error is logged as well
    /// as returned.
    fn logging_errors(
        &self,
        future: impl std::future::Future<Output = Result<(), CommandError>> + Send + 'static,
    ) -> *mut Call {
        let console = self.console.clone().expect("a connection has the console");
        let id = self.id;
        call::start(
            None,
            async move {
                let result = future.await;
                if let Err(e) = &result {
                    let line = Line::error(e);
                    console.send(move |console| console.append(id, vec![line]));
                }
                result
            },
            |_, ()| {},
        )
    }
}

#[derive(Default)]
pub struct DoctorFixRust {
    id: QString,
    label: QVariantMap,
    confirm_label: QVariantMap,
    description: QVariantMap,
}

#[derive(Default)]
pub struct DoctorReportRust {
    translation_key: QString,
    message: QString,
    message_args: QVariantMap,
    fixes: QObjectList,
    report_id: Uuid,
    conn_id: Option<ConnectionId>,
}

impl qobject::DoctorReport {
    fn choose(&self, fix: &QString) -> *mut Call {
        let (report, choice) = (self.report_id, fix.to_string());
        let conn_id = self.conn_id.expect("a report has its connection");
        call::start(
            None,
            async move {
                let conn = super::app()
                    .state::<IpcState>()
                    .get_conn(conn_id)
                    .ok_or_else(|| anyhow::anyhow!("No such connection: {conn_id}"))?;
                conn.send_async(S2CMessage::PatientResponse { id: report, choice })
                    .await
                    .map_err(anyhow::Error::from)?;
                Ok(())
            },
            |_, ()| {},
        )
    }
}

fn to_qmap(map: Option<&HashMap<String, String>>) -> QVariantMap {
    map.into_iter()
        .flatten()
        .map(|(key, value)| (QString::from(key), QVariant::from(&QString::from(value))))
        .collect()
}

pub struct ConsoleRust {
    connections: QObjectList,
    focused: *mut Connection,
    doctor_reports: QObjectList,
    by_id: BTreeMap<i32, Child<Connection>>,
    /// The lines to add to each connection's log, together.
    pending: BTreeMap<i32, Vec<Line>>,
    outbox: Option<Arc<Outbox<Console>>>,
}

impl Default for ConsoleRust {
    fn default() -> Self {
        Self {
            connections: QObjectList::default(),
            focused: std::ptr::null_mut(),
            doctor_reports: QObjectList::default(),
            by_id: BTreeMap::new(),
            pending: BTreeMap::new(),
            outbox: None,
        }
    }
}

impl cxx_qt::Initialize for Console {
    fn initialize(mut self: Pin<&mut Self>) {
        let outbox = Outbox::new(self.qt_thread());
        self.as_mut().rust_mut().outbox = Some(outbox.clone());
        let app = super::app();
        // the games already connected
        for id in app.state::<IpcState>().get_conns() {
            self.as_mut().connection(id.to_qml(), QString::default());
        }
        app.ipc_events()
            .subscribe(move |event| outbox.send(move |console| console.handle(event)));
    }
}

impl Console {
    fn allocate(self: Pin<&mut Self>, profile_id: &QString) -> *mut Connection {
        let id = super::app().state::<IpcState>().alloc();
        self.connection(id.to_qml(), profile_id.clone())
    }

    fn dismiss_doctor_report(mut self: Pin<&mut Self>, index: i32) {
        let Ok(index) = usize::try_from(index) else {
            return;
        };
        let mut reports = self.doctor_reports.0.iter().cloned().collect::<Vec<_>>();
        if index < reports.len() {
            reports.remove(index);
            self.as_mut().rust_mut().doctor_reports = reports.into_iter().collect();
            self.doctor_reports_changed();
        }
    }

    /// The connection with an id, which is made if it's unknown, as a game can connect without
    /// having been launched from the app.
    fn connection(mut self: Pin<&mut Self>, id: i32, profile_id: QString) -> *mut Connection {
        if let Some(connection) = self.by_id.get(&id) {
            return connection.as_ptr();
        }
        let outbox = self.outbox.clone();
        let mut connection = qobject::new_connection();
        let log = Child::adopt(connection.pin_mut(), qobject::new_console_log());
        {
            let mut rust = connection.pin_mut().rust_mut();
            rust.id = id;
            rust.profile_id = profile_id;
            rust.log = log.as_ptr();
            rust.log_object = Some(log);
            rust.console = outbox;
        }
        let connection = Child::adopt(self.as_mut(), connection);
        let pointer = connection.as_ptr();
        let mut rust = self.as_mut().rust_mut();
        rust.by_id.insert(id, connection);
        rust.connections = rust.by_id.values().map(Child::as_qobject).collect();
        self.connections_changed();
        pointer
    }

    fn connection_mut(mut self: Pin<&mut Self>, id: i32) -> Pin<&mut Connection> {
        self.as_mut().connection(id, QString::default());
        let rust = self.rust_mut().get_mut();
        rust.by_id
            .get_mut(&id)
            .expect("the connection was made")
            .get_mut()
    }

    fn handle(mut self: Pin<&mut Self>, event: Event) {
        match event {
            Event::Message { conn_id, msg } => {
                let id = conn_id.to_qml();
                if let C2SMessage::DoctorReport(report) = msg {
                    self.add_report(conn_id, report);
                    return;
                }
                if matches!(msg, C2SMessage::Connect { .. }) {
                    self.as_mut()
                        .connection_mut(id)
                        .set_status(ConnectionStatus::Connected);
                }
                if let Some(line) = Line::of(&msg) {
                    self.append(id, vec![line]);
                }
            }
            Event::Closed { conn_id } => {
                if self.by_id.contains_key(&conn_id.to_qml()) {
                    self.connection_mut(conn_id.to_qml())
                        .set_status(ConnectionStatus::Disconnected);
                }
            }
        }
    }

    /// Adds lines to a connection's log once what's being delivered has been, with any others.
    fn append(mut self: Pin<&mut Self>, id: i32, lines: Vec<Line>) {
        self.as_mut().connection(id, QString::default());
        let first = self.pending.is_empty();
        self.as_mut()
            .rust_mut()
            .pending
            .entry(id)
            .or_default()
            .extend(lines);
        if first {
            // fails only once the console has been destroyed
            _ = self.qt_thread().queue(|console| console.flush());
        }
    }

    fn flush(mut self: Pin<&mut Self>) {
        let pending = std::mem::take(&mut self.as_mut().rust_mut().pending);
        for (id, lines) in pending {
            self.as_mut().connection_mut(id).append(lines);
        }
    }

    fn add_report(mut self: Pin<&mut Self>, conn_id: ConnectionId, report: DoctorReport) {
        let mut object = qobject::new_doctor_report();
        let mut fixes = QObjectList::default();
        for fix in &report.fixes {
            let mut fix_object = qobject::new_doctor_fix();
            {
                let mut rust = fix_object.pin_mut().rust_mut();
                rust.id = QString::from(&fix.id);
                rust.label = to_qmap(fix.label.as_ref());
                rust.confirm_label = to_qmap(fix.confirm_label.as_ref());
                rust.description = to_qmap(fix.description.as_ref());
            }
            fixes
                .0
                .append(Child::adopt(object.pin_mut(), fix_object).as_qobject());
        }
        {
            let mut rust = object.pin_mut().rust_mut();
            rust.translation_key = QString::from(&report.translation_key);
            rust.message = report
                .message
                .as_deref()
                .map(QString::from)
                .unwrap_or_default();
            rust.message_args = to_qmap(report.message_args.as_ref());
            rust.fixes = fixes;
            rust.report_id = report.id;
            rust.conn_id = Some(conn_id);
        }
        let report = Child::adopt(self.as_mut(), object);
        self.as_mut()
            .rust_mut()
            .doctor_reports
            .0
            .append(report.as_qobject());
        self.doctor_reports_changed();
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use super::*;

    #[test]
    fn shows_logs_by_level() {
        let line = Line::of(&C2SMessage::Log {
            level: LogLevel::Warning,
            scope: "BepInEx".to_owned(),
            message: "Careful".to_owned(),
        })
        .unwrap();
        assert_eq!(
            (line.kind.as_str(), line.data_type.as_str()),
            ("WARN", "WARN")
        );
        assert_eq!(
            (line.scope.as_str(), line.filter.as_str()),
            ("BepInEx:", "WARN")
        );
        assert!(line.searchable);
    }

    #[test]
    fn shows_output() {
        let unicode = Line::of(&C2SMessage::Output {
            channel: StandardOutputChannel::Err,
            line: OutputLine::Unicode("oops".to_owned()),
        })
        .unwrap();
        assert_eq!(
            (unicode.kind.as_str(), unicode.filter.as_str()),
            ("STDERR", "STDERR")
        );
        assert_eq!(unicode.message, "oops");
        assert!(unicode.searchable);

        let bytes = Line::of(&C2SMessage::Output {
            channel: StandardOutputChannel::Out,
            line: OutputLine::Bytes(vec![0xff, 0]),
        })
        .unwrap();
        assert_eq!(bytes.message, "[255,0]");
        assert!(!bytes.searchable);
    }

    #[test]
    fn shows_the_command() {
        let line = Line::of(&C2SMessage::Start {
            command: SafeOsString::Unicode("game".to_owned()),
            args: vec![SafeOsString::Unicode("--flag".to_owned())],
            env: HashMap::from([
                ("B".to_owned(), SafeOsString::Unicode("2".to_owned())),
                ("A".to_owned(), SafeOsString::NonUnicodeBytes(vec![0xff])),
            ]),
        })
        .unwrap();
        assert_eq!(line.kind, "START");
        assert_eq!(
            line.message,
            r#"A={"NonUnicodeBytes":[255]} B="2" "game"  "--flag""#
        );
    }

    #[test]
    fn shows_the_process() {
        let started = Line::of(&C2SMessage::Started {
            pid: NonZeroU32::new(42).unwrap(),
        })
        .unwrap();
        assert_eq!(started.message, "Game process started with id 42");
        let exit = Line::of(&C2SMessage::Exit { code: None }).unwrap();
        assert_eq!(exit.message, "Unknown exit code");
        assert_eq!(
            Line::of(&C2SMessage::Exit { code: Some(1) })
                .unwrap()
                .message,
            "1"
        );
    }
}
