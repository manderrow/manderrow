// Generated from src/locales/en-CA.json by `just gen` and the build. Edit that instead.
pragma Singleton
import QtQuick
import Manderrow

// The translations, as typed properties: `Strings.game_select.title` is
// `I18n.t("game_select.title")`, and a translation with placeholders is a function of them,
// like `Strings.status_bar.downloads_tracker(count)`. Like `I18n.t`, they follow the locale.
QtObject {
    readonly property StringsGlobal global: StringsGlobal {}
    readonly property StringsSplashscreen splashscreen: StringsSplashscreen {}
    readonly property StringsTitlebar titlebar: StringsTitlebar {}
    readonly property StringsTaskManager task_manager: StringsTaskManager {}
    readonly property StringsStatusBar status_bar: StringsStatusBar {}
    readonly property Strings404Page _404_page: Strings404Page {}
    readonly property StringsGameSelect game_select: StringsGameSelect {}
    readonly property StringsProfile profile: StringsProfile {}
    readonly property StringsModlist modlist: StringsModlist {}
    readonly property StringsDoctor doctor: StringsDoctor {}
    readonly property StringsSettings settings: StringsSettings {}
    readonly property StringsConsole console_: StringsConsole {}
    readonly property StringsError error: StringsError {}

    component StringsGlobalPhrases: QtObject {
        /// "OK"
        readonly property string ok: I18n.t("global.phrases.ok")
        /// "Yes"
        readonly property string yes: I18n.t("global.phrases.yes")
        /// "No"
        readonly property string no: I18n.t("global.phrases.no")
        /// "Delete"
        readonly property string delete_: I18n.t("global.phrases.delete")
        /// "Confirm"
        readonly property string confirm: I18n.t("global.phrases.confirm")
        /// "Cancel"
        readonly property string cancel: I18n.t("global.phrases.cancel")
        /// "Loading"
        readonly property string loading: I18n.t("global.phrases.loading")
        /// "Search"
        readonly property string search: I18n.t("global.phrases.search")
        /// "Select"
        readonly property string select: I18n.t("global.phrases.select")
        /// "Apply"
        readonly property string apply: I18n.t("global.phrases.apply")
        /// "Deselect"
        readonly property string deselect: I18n.t("global.phrases.deselect")
        /// "Select All"
        readonly property string select_all: I18n.t("global.phrases.select_all")
        /// "Deselect All"
        readonly property string deselect_all: I18n.t("global.phrases.deselect_all")
        /// "Back"
        readonly property string back: I18n.t("global.phrases.back")
        /// "Next"
        readonly property string next: I18n.t("global.phrases.next")
        /// "Previous"
        readonly property string previous: I18n.t("global.phrases.previous")
        /// "Close"
        readonly property string close: I18n.t("global.phrases.close")
        /// "Remove"
        readonly property string remove: I18n.t("global.phrases.remove")
        /// "Remove All"
        readonly property string remove_all: I18n.t("global.phrases.remove_all")
        /// "Edit"
        readonly property string edit: I18n.t("global.phrases.edit")
    }

    component StringsGlobalGameSortColumn: QtObject {
        /// "Name"
        readonly property string name: I18n.t("global.game_sort_column.name")
        /// "Mod Downloads"
        readonly property string mod_downloads: I18n.t("global.game_sort_column.mod_downloads")
        /// "Popularity"
        readonly property string popularity: I18n.t("global.game_sort_column.popularity")
    }

    component StringsGlobalModSortColumn: QtObject {
        /// "Relevance"
        readonly property string relevance: I18n.t("global.mod_sort_column.relevance")
        /// "Downloads"
        readonly property string downloads: I18n.t("global.mod_sort_column.downloads")
        /// "Name"
        readonly property string name: I18n.t("global.mod_sort_column.name")
        /// "Owner"
        readonly property string owner: I18n.t("global.mod_sort_column.owner")
        /// "Size"
        readonly property string size: I18n.t("global.mod_sort_column.size")
    }

    component StringsGlobalSortOrder: QtObject {
        /// "Ascending"
        readonly property string ascending: I18n.t("global.sort_order.ascending")
        /// "Descending"
        readonly property string descending: I18n.t("global.sort_order.descending")
    }

    component StringsGlobalSortType: QtObject {
        /// "Alphabetical"
        readonly property string alphabetical: I18n.t("global.sort_type.alphabetical")
        /// "Creation Date"
        readonly property string creation_date: I18n.t("global.sort_type.creation_date")
    }

    component StringsGlobalSelectDropdown: QtObject {
        /// "Select..."
        readonly property string default_fallback: I18n.t("global.select_dropdown.default_fallback")
        /// "Sort By"
        readonly property string sort_by: I18n.t("global.select_dropdown.sort_by")
    }

    component StringsGlobal: QtObject {
        readonly property StringsGlobalPhrases phrases: StringsGlobalPhrases {}
        readonly property StringsGlobalGameSortColumn game_sort_column: StringsGlobalGameSortColumn {}
        readonly property StringsGlobalModSortColumn mod_sort_column: StringsGlobalModSortColumn {}
        /// "Sort"
        readonly property string sort_title: I18n.t("global.sort_title")
        readonly property StringsGlobalSortOrder sort_order: StringsGlobalSortOrder {}
        readonly property StringsGlobalSortType sort_type: StringsGlobalSortType {}
        readonly property StringsGlobalSelectDropdown select_dropdown: StringsGlobalSelectDropdown {}
        /// "Confirm Decision?"
        readonly property string dialogue_default_title: I18n.t("global.dialogue_default_title")
    }

    component StringsSplashscreen: QtObject {
        /// "Launching..."
        readonly property string launching_msg: I18n.t("splashscreen.launching_msg")
    }

    component StringsTitlebar: QtObject {
        /// "Minimize"
        readonly property string minimize_btn: I18n.t("titlebar.minimize_btn")
        /// "Maximize"
        readonly property string maximize_btn: I18n.t("titlebar.maximize_btn")
        /// "Restore"
        readonly property string restore_btn: I18n.t("titlebar.restore_btn")
    }

    component StringsTaskManager: QtObject {
        /// "Tasks"
        readonly property string title: I18n.t("task_manager.title")
        /// "Active"
        readonly property string active_tab_name: I18n.t("task_manager.active_tab_name")
        /// "Completed"
        readonly property string completed_tab_name: I18n.t("task_manager.completed_tab_name")
        /// "Source"
        readonly property string source_label: I18n.t("task_manager.source_label")
        /// "No tasks yet."
        readonly property string no_tasks_yet_msg: I18n.t("task_manager.no_tasks_yet_msg")
    }

    component StringsStatusBar: QtObject {
        /// "{{ count }} downloads"
        function downloads_tracker(count: var): string {
            return I18n.t("status_bar.downloads_tracker", { count: count });
        }
        /// "{{ count }} other tasks"
        function other_tasks_tracker(count: var): string {
            return I18n.t("status_bar.other_tasks_tracker", { count: count });
        }
    }

    component Strings404Page: QtObject {
        /// "There is nothing on this page. You somehow got here..."
        readonly property string subtitle: I18n.t("404_page.subtitle")
        /// "Go back home"
        readonly property string home_btn: I18n.t("404_page.home_btn")
    }

    component StringsGameSelectSearch: QtObject {
        /// "Search for a game"
        readonly property string input_placeholder: I18n.t("game_select.search.input_placeholder")
        /// "Change to {{ type }} layout"
        function display_type_btn(type: var): string {
            return I18n.t("game_select.search.display_type_btn", { type: type });
        }
        /// "card"
        readonly property string card_display_type: I18n.t("game_select.search.card_display_type")
        /// "list"
        readonly property string list_display_type: I18n.t("game_select.search.list_display_type")
    }

    component StringsGameSelect: QtObject {
        /// "Game Selection"
        readonly property string title: I18n.t("game_select.title")
        /// "Select the game you are managing mods for"
        readonly property string subtitle: I18n.t("game_select.subtitle")
        readonly property StringsGameSelectSearch search: StringsGameSelectSearch {}
        /// "No games matched the search"
        readonly property string no_games_msg: I18n.t("game_select.no_games_msg")
        /// "Background image of {{ gameName }}"
        function bg_img_alt(gameName: var): string {
            return I18n.t("game_select.bg_img_alt", { gameName: gameName });
        }
        /// "Select"
        readonly property string select_btn: I18n.t("game_select.select_btn")
        /// "Set Default"
        readonly property string set_default_btn: I18n.t("game_select.set_default_btn")
        /// "Favorite this game"
        readonly property string fav_btn: I18n.t("game_select.fav_btn")
    }

    component StringsProfileBlank: QtObject {
        /// "No profiles found"
        readonly property string no_profiles_title: I18n.t("profile.blank.no_profiles_title")
        /// "Create a profile to get started"
        readonly property string no_profiles_subtitle: I18n.t("profile.blank.no_profiles_subtitle")
        /// "Create Profile"
        readonly property string create_profile_btn: I18n.t("profile.blank.create_profile_btn")
    }

    component StringsProfileImportModel: QtObject {
        /// "Import Modlist"
        readonly property string import_title: I18n.t("profile.import_model.import_title")
        /// "Preview Mods"
        readonly property string preview_title: I18n.t("profile.import_model.preview_title")
        /// "Thunderstore Code"
        readonly property string thunderstore_code_label: I18n.t("profile.import_model.thunderstore_code_label")
    }

    component StringsProfileSidebar: QtObject {
        /// "Kill game"
        readonly property string kill_game_btn: I18n.t("profile.sidebar.kill_game_btn")
        /// "Cancel"
        readonly property string cancel_launch_btn: I18n.t("profile.sidebar.cancel_launch_btn")
        /// "Launch profile"
        readonly property string launch_modded_btn: I18n.t("profile.sidebar.launch_modded_btn")
        /// "Start vanilla"
        readonly property string launch_vanilla_btn: I18n.t("profile.sidebar.launch_vanilla_btn")
        /// "Game Settings"
        readonly property string game_settings_btn: I18n.t("profile.sidebar.game_settings_btn")
        /// "Profiles"
        readonly property string profiles_title: I18n.t("profile.sidebar.profiles_title")
        /// "Create new profile"
        readonly property string create_profile_tooltip: I18n.t("profile.sidebar.create_profile_tooltip")
        /// "Import profile"
        readonly property string import_profile_tooltip: I18n.t("profile.sidebar.import_profile_tooltip")
        /// "No profiles found."
        readonly property string no_profiles_search_msg: I18n.t("profile.sidebar.no_profiles_search_msg")
        /// "Profile is running"
        readonly property string profile_running_icon: I18n.t("profile.sidebar.profile_running_icon")
        /// "More"
        readonly property string ellipsis_btn: I18n.t("profile.sidebar.ellipsis_btn")
        /// "Pin"
        readonly property string pin_profile_btn: I18n.t("profile.sidebar.pin_profile_btn")
        /// "Unpin"
        readonly property string unpin_profile_btn: I18n.t("profile.sidebar.unpin_profile_btn")
        /// "Rename"
        readonly property string rename_profile_btn: I18n.t("profile.sidebar.rename_profile_btn")
        /// "Duplicate"
        readonly property string duplicate_profile_btn: I18n.t("profile.sidebar.duplicate_profile_btn")
        /// "Copy UUID"
        readonly property string copy_id_profile_btn: I18n.t("profile.sidebar.copy_id_profile_btn")
        /// "Open Folder"
        readonly property string open_folder_profile_btn: I18n.t("profile.sidebar.open_folder_profile_btn")
        /// "Share"
        readonly property string share_profile_btn: I18n.t("profile.sidebar.share_profile_btn")
    }

    component StringsProfileTabs: QtObject {
        /// "Installed Mods"
        readonly property string installed: I18n.t("profile.tabs.installed")
        /// "Online Mods"
        readonly property string online: I18n.t("profile.tabs.online")
        /// "Config"
        readonly property string config: I18n.t("profile.tabs.config")
    }

    component StringsProfileModMarkdown: QtObject {
        /// "No README provided."
        readonly property string no_readme_provided: I18n.t("profile.mod_markdown.no_readme_provided")
        /// "No changelog provided."
        readonly property string no_changelog_provided: I18n.t("profile.mod_markdown.no_changelog_provided")
    }

    component StringsProfile: QtObject {
        readonly property StringsProfileBlank blank: StringsProfileBlank {}
        readonly property StringsProfileImportModel import_model: StringsProfileImportModel {}
        readonly property StringsProfileSidebar sidebar: StringsProfileSidebar {}
        /// "Are you sure you want to delete the profile {{ profileName }}? This action cannot be undone."
        function delete_msg(profileName: var): string {
            return I18n.t("profile.delete_msg", { profileName: profileName });
        }
        readonly property StringsProfileTabs tabs: StringsProfileTabs {}
        /// "New profile"
        readonly property string default_profile_name: I18n.t("profile.default_profile_name")
        readonly property StringsProfileModMarkdown mod_markdown: StringsProfileModMarkdown {}
    }

    component StringsModlistInstalled: QtObject {
        /// "No mods installed"
        readonly property string no_mods_title: I18n.t("modlist.installed.no_mods_title")
        /// "No mods are installed for the current profile yet."
        readonly property string no_mods_msg: I18n.t("modlist.installed.no_mods_msg")
        /// "Browse Mods"
        readonly property string browse_btn: I18n.t("modlist.installed.browse_btn")
        /// "Check for Updates"
        readonly property string check_updates_btn: I18n.t("modlist.installed.check_updates_btn")
        /// "Updates Available"
        readonly property string updates_available_btn: I18n.t("modlist.installed.updates_available_btn")
        /// "Bulk Actions"
        readonly property string multiselect_title: I18n.t("modlist.installed.multiselect_title")
        /// "Mod Name"
        readonly property string multiselect_mod_name: I18n.t("modlist.installed.multiselect_mod_name")
        /// "Size"
        readonly property string multiselect_size: I18n.t("modlist.installed.multiselect_size")
        /// "{{ count }} mod selected"
        function selected_count_single(count: var): string {
            return I18n.t("modlist.installed.selected_count_single", { count: count });
        }
        /// "{{ count }} mods selected"
        function selected_count_plural(count: var): string {
            return I18n.t("modlist.installed.selected_count_plural", { count: count });
        }
        /// "Enable All"
        readonly property string enable_all: I18n.t("modlist.installed.enable_all")
        /// "Disable All"
        readonly property string disable_all: I18n.t("modlist.installed.disable_all")
        /// "Delete All"
        readonly property string delete_all: I18n.t("modlist.installed.delete_all")
        /// "Update All"
        readonly property string update_all: I18n.t("modlist.installed.update_all")
        /// "Select mods to update"
        readonly property string updater_title: I18n.t("modlist.installed.updater_title")
        /// "Update Selected"
        readonly property string update_selected_btn: I18n.t("modlist.installed.update_selected_btn")
        /// "Update All"
        readonly property string update_all_btn: I18n.t("modlist.installed.update_all_btn")
        /// "Change Version"
        readonly property string change_version_btn: I18n.t("modlist.installed.change_version_btn")
        /// "Select Version"
        readonly property string select_version_label: I18n.t("modlist.installed.select_version_label")
        /// "Search version"
        readonly property string search_version_placeholder: I18n.t("modlist.installed.search_version_placeholder")
        /// "Uninstall"
        readonly property string uninstall_btn: I18n.t("modlist.installed.uninstall_btn")
    }

    component StringsModlistOnline: QtObject {
        /// "No mods found."
        readonly property string no_mods: I18n.t("modlist.online.no_mods")
        /// "Install"
        readonly property string install_btn: I18n.t("modlist.online.install_btn")
    }

    component StringsModlistModview: QtObject {
        /// "No mod selected"
        readonly property string no_mod_selected_title: I18n.t("modlist.modview.no_mod_selected_title")
        /// "Select a mod to view it here."
        readonly property string no_mod_selected_subtitle: I18n.t("modlist.modview.no_mod_selected_subtitle")
        /// "Dependencies"
        readonly property string dependencies_title: I18n.t("modlist.modview.dependencies_title")
        /// "Versions"
        readonly property string versions_label: I18n.t("modlist.modview.versions_label")
        /// "Changelog"
        readonly property string changelog_title: I18n.t("modlist.modview.changelog_title")
        /// "Donate"
        readonly property string donate_btn: I18n.t("modlist.modview.donate_btn")
        /// "This mod has no dependencies."
        readonly property string no_dependencies_msg: I18n.t("modlist.modview.no_dependencies_msg")
    }

    component StringsModlistSearch: QtObject {
        /// "Search for a mod"
        readonly property string input_placeholder: I18n.t("modlist.search.input_placeholder")
    }

    component StringsModlist: QtObject {
        /// "Fetching mods..."
        readonly property string fetching_msg: I18n.t("modlist.fetching_msg")
        /// "Discovered {{ count }} mods"
        function discovered_msg(count: var): string {
            return I18n.t("modlist.discovered_msg", { count: count });
        }
        readonly property StringsModlistInstalled installed: StringsModlistInstalled {}
        readonly property StringsModlistOnline online: StringsModlistOnline {}
        readonly property StringsModlistModview modview: StringsModlistModview {}
        readonly property StringsModlistSearch search: StringsModlistSearch {}
    }

    component StringsDoctorLaunchOptionsFixesApply: QtObject {
        /// "Do it for me"
        readonly property string label: I18n.t("doctor.launch_options.fixes.apply.label")
        /// "Apply Changes"
        readonly property string confirm_label: I18n.t("doctor.launch_options.fixes.apply.confirm_label")
        /// "We'll go ahead and apply the changes for you. If you have it open, Steam will be closed as part of this process."
        readonly property string description: I18n.t("doctor.launch_options.fixes.apply.description")
    }

    component StringsDoctorLaunchOptionsFixesRetry: QtObject {
        /// "Do it myself"
        readonly property string label: I18n.t("doctor.launch_options.fixes.retry.label")
        /// "I did it"
        readonly property string confirm_label: I18n.t("doctor.launch_options.fixes.retry.confirm_label")
        /// "You'll need to paste the following string in the Launch Options field for the game in Steam: {{ launch_options }}"
        function description(launch_options: var): string {
            return I18n.t("doctor.launch_options.fixes.retry.description", { launch_options: launch_options });
        }
    }

    component StringsDoctorLaunchOptionsFixesIgnore: QtObject {
        /// "Trust me, I know what I'm doing"
        readonly property string label: I18n.t("doctor.launch_options.fixes.ignore.label")
        /// "Ignore"
        readonly property string confirm_label: I18n.t("doctor.launch_options.fixes.ignore.confirm_label")
        /// "Whatever you say, pal. You're on your own!"
        readonly property string description: I18n.t("doctor.launch_options.fixes.ignore.description")
    }

    component StringsDoctorLaunchOptionsFixesAbort: QtObject {
        /// "I don't want to"
        readonly property string label: I18n.t("doctor.launch_options.fixes.abort.label")
        /// "Abort"
        readonly property string confirm_label: I18n.t("doctor.launch_options.fixes.abort.confirm_label")
        /// "No worries. Unfortunately, you'll be unable to launch with Manderrow at this time."
        readonly property string description: I18n.t("doctor.launch_options.fixes.abort.description")
    }

    component StringsDoctorLaunchOptionsFixes: QtObject {
        readonly property StringsDoctorLaunchOptionsFixesApply apply: StringsDoctorLaunchOptionsFixesApply {}
        readonly property StringsDoctorLaunchOptionsFixesRetry retry: StringsDoctorLaunchOptionsFixesRetry {}
        readonly property StringsDoctorLaunchOptionsFixesIgnore ignore: StringsDoctorLaunchOptionsFixesIgnore {}
        readonly property StringsDoctorLaunchOptionsFixesAbort abort: StringsDoctorLaunchOptionsFixesAbort {}
    }

    component StringsDoctorLaunchOptions: QtObject {
        /// "To unlock the full functionality of Manderrow, one small tweak must be made to your Steam configuration."
        readonly property string message: I18n.t("doctor.launch_options.message")
        /// "It seems you've set some Launch Options for this game in Steam. To unlock the full functionality of Manderrow, those Launch Options will need to be changed."
        readonly property string message_overwrite: I18n.t("doctor.launch_options.message_overwrite")
        readonly property StringsDoctorLaunchOptionsFixes fixes: StringsDoctorLaunchOptionsFixes {}
    }

    component StringsDoctor: QtObject {
        readonly property StringsDoctorLaunchOptions launch_options: StringsDoctorLaunchOptions {}
    }

    component StringsSettingsSection: QtObject {
        /// "General"
        readonly property string general: I18n.t("settings.section.general")
        /// "Appearance"
        readonly property string appearance: I18n.t("settings.section.appearance")
        /// "Launching"
        readonly property string launching: I18n.t("settings.section.launching")
    }

    component StringsSettingsSettings: QtObject {
        /// "Default game"
        readonly property string defaultGame: I18n.t("settings.settings.defaultGame")
        /// "Theme"
        readonly property string theme: I18n.t("settings.settings.theme")
        /// "Translucent window"
        readonly property string windowTranslucent: I18n.t("settings.settings.windowTranslucent")
        /// "Window opacity"
        readonly property string windowOpacity: I18n.t("settings.settings.windowOpacity")
        /// "Blur behind window"
        readonly property string windowBlur: I18n.t("settings.settings.windowBlur")
        /// "Open console on launch?"
        readonly property string openConsoleOnLaunch: I18n.t("settings.settings.openConsoleOnLaunch")
    }

    component StringsSettingsOptionsTheme: QtObject {
        /// "Auto"
        readonly property string auto: I18n.t("settings.options.theme.auto")
        /// "Dark"
        readonly property string dark: I18n.t("settings.options.theme.dark")
        /// "Light"
        readonly property string light: I18n.t("settings.options.theme.light")
    }

    component StringsSettingsOptions: QtObject {
        readonly property StringsSettingsOptionsTheme theme: StringsSettingsOptionsTheme {}
    }

    component StringsSettings: QtObject {
        /// "Settings"
        readonly property string title: I18n.t("settings.title")
        /// "App Settings"
        readonly property string link_title: I18n.t("settings.link_title")
        /// "Search settings..."
        readonly property string search_input_placeholder: I18n.t("settings.search_input_placeholder")
        readonly property StringsSettingsSection section: StringsSettingsSection {}
        readonly property StringsSettingsSettings settings: StringsSettingsSettings {}
        readonly property StringsSettingsOptions options: StringsSettingsOptions {}
    }

    component StringsConsole: QtObject {
        /// "Search log..."
        readonly property string search_input_placeholder: I18n.t("console.search_input_placeholder")
        /// "Launching game, please wait..."
        readonly property string launching_log_placeholder: I18n.t("console.launching_log_placeholder")
        /// "Created at"
        readonly property string log_created_at: I18n.t("console.log_created_at")
        /// "Game not running."
        readonly property string game_not_running: I18n.t("console.game_not_running")
        /// "Live log"
        readonly property string live_log: I18n.t("console.live_log")
        /// "Connected"
        readonly property string live_log_connected: I18n.t("console.live_log_connected")
        /// "Disconnected"
        readonly property string live_log_disconnected: I18n.t("console.live_log_disconnected")
    }

    component StringsError: QtObject {
        /// "Oops! An error occurred..."
        readonly property string title: I18n.t("error.title")
        /// "But don't worry, we caught it for you. We're still in beta, so bugs that can cause errors are expected. Details are below:"
        readonly property string deescalation_msg: I18n.t("error.deescalation_msg")
        /// "If you would like to help us fix this, please report the error to us below. Your help is greatly appreciated!"
        readonly property string report_msg: I18n.t("error.report_msg")
        /// "Otherwise, you can try to ignore this error and continue using Manderrow for now. Depending on the severity though, this may not be possible."
        readonly property string ignore_msg: I18n.t("error.ignore_msg")
        /// "Native Stack Trace"
        readonly property string native_stack_trace: I18n.t("error.native_stack_trace")
        /// "JavaScript Stack Trace"
        readonly property string js_stack_trace: I18n.t("error.js_stack_trace")
        /// "Report"
        readonly property string report_btn: I18n.t("error.report_btn")
        /// "Ignore"
        readonly property string ignore_btn: I18n.t("error.ignore_btn")
    }
}
