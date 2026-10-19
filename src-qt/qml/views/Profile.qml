import QtQuick
import Manderrow

// The /profile/:gameId?/:profileId? route, ported from Profile.tsx.
Item {
    id: profile

    // whether the game selection screen should show, and whether it is still showing (it
    // animates away after it should no longer show)
    property bool gameSelectShouldShow: Router.gameId === undefined
    property bool gameSelectShowing: Router.gameId === undefined

    Component.onCompleted: {
        if (Globals.initialGame)
            Router.navigate(`/profile/${Globals.initialGame}`, { replace: true });
    }

    Loader {
        anchors.fill: parent
        active: Router.gameId !== undefined
        sourceComponent: ProfileWithGame {
            gameId: Router.gameId
            profileId: Router.profileId
            gameSelectShowing: profile.gameSelectShowing
            onShowGameSelect: {
                profile.gameSelectShouldShow = true;
                profile.gameSelectShowing = true;
            }
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
