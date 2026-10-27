pragma ComponentBehavior: Bound
import QtQuick
import Manderrow

// The /profile/:gameId?/:profileId? route, ported from Profile.tsx.
Item {
    id: profile

    // whether the game selection screen should show, and whether it is still showing (it
    // animates away after it should no longer show). They're set rather than bound to the route,
    // as bindings on the route can update in either order, so the screen could be created with
    // the other's old value.
    property bool gameSelectShouldShow: false
    property bool gameSelectShowing: false

    function showGameSelect() {
        // in this order, so that the screen is created already shown
        gameSelectShouldShow = true;
        gameSelectShowing = true;
    }

    Component.onCompleted: {
        if (Settings.defaultGame !== "")
            Router.navigate(`/profile/${Settings.defaultGame}`, { replace: true });
        else if (Router.gameId === undefined)
            showGameSelect();
    }

    Connections {
        target: Router
        function onNavigated() {
            // leaving the route, for another that has no game either
            if (Router.route !== "profile")
                return;
            if (Router.gameId === undefined)
                profile.showGameSelect();
            else
                // a game was chosen, on the screen or by going back
                profile.gameSelectShouldShow = false;
        }
    }

    Loader {
        anchors.fill: parent
        active: Router.gameId !== undefined
        // the game selection screen covers it, and would show it through its translucency
        visible: !profile.gameSelectShouldShow
        sourceComponent: ProfileWithGame {
            gameId: Router.gameId
            profileId: Router.profileId
            gameSelectShowing: profile.gameSelectShowing
            onShowGameSelect: profile.showGameSelect()
        }
    }

    Loader {
        anchors.fill: parent
        active: profile.gameSelectShowing
        z: 1
        sourceComponent: GameSelect {
            replace: true
            shouldShow: profile.gameSelectShouldShow
            onBeginDismiss: profile.gameSelectShouldShow = false
            onFinishDismiss: profile.gameSelectShowing = false
        }
    }
}
