pragma Singleton
import QtQuick
import Manderrow

// Tracks backend tasks and their progress, ported from api/tasks.ts.
//
// A task is a QtObject with `metadata`, `status`, `progress`, `dependencies`, and `isComplete`
// properties that bindings can depend on. A listener is a function receiving events shaped like
// `{ event: "created" | "progress" | "dependency" | "dropped", ... }`.
QtObject {
    id: tasks

    readonly property var kind: ({ aggregate: "Aggregate", download: "Download", other: "Other" })
    readonly property var progressUnit: ({ bytes: "Bytes", other: "Other" })

    property var _tasks: ({})
    /// Every known task, in creation order.
    property var tasksArray: []

    readonly property Component _taskComponent: Component {
        QtObject {
            property int taskId
            property var metadata: ({ title: "", kind: "Other", progress_unit: "Other" })
            property var status: ({ status: "Unstarted" })
            property var progress: ({ completed: 0, total: 0 })
            property var dependencies: []
            readonly property bool isComplete: status.status === "Success" || status.status === "Failed" || status.status === "Cancelled"

            property var listeners: []
            /// Used by notifyTaskListeners and unregisterTaskListener.
            property int listenerIndex: 0
        }
    }

    function initProgress() {
        return { completed: 0, total: 0 };
    }

    function get(id) {
        return _tasks[id];
    }

    function _create(id, props) {
        const task = _taskComponent.createObject(tasks, Object.assign({ taskId: id }, props));
        _tasks[id] = task;
        tasksArray = tasksArray.concat([task]);
        return task;
    }

    function registerTaskListener(id, listener) {
        const task = _tasks[id];
        if (task === undefined)
            _create(id, { listeners: [listener] });
        else
            task.listeners.push(listener);
    }

    function unregisterTaskListener(id, listener) {
        const task = _tasks[id];
        if (task === undefined)
            return;
        const i = task.listeners.indexOf(listener);
        if (i !== -1)
            task.listeners.splice(i, 1);
        // adjust so that if removed during iteration, subsequent listeners are not skipped
        if (task.listenerIndex >= i)
            task.listenerIndex--;
    }

    function _notify(task, event) {
        for (task.listenerIndex = 0; task.listenerIndex < task.listeners.length; task.listenerIndex++) {
            try {
                task.listeners[task.listenerIndex](event);
            } catch (e) {
                console.error("Error in task listener:", e, e.stack);
            }
        }
    }

    readonly property var _subscriptions: [
        Api.listen("task_created", payload => {
            const task = tasks._tasks[payload.id];
            if (task === undefined) {
                tasks._create(payload.id, { metadata: payload.metadata, status: { status: "Running" } });
                return;
            }
            if (task.status.status !== "Unstarted") {
                console.error(`Duplicate id ${payload.id} in TaskCreatedEvent`, JSON.stringify(payload.metadata));
                return;
            }
            task.metadata = payload.metadata;
            task.status = { status: "Running" };
            tasks._notify(task, { event: "created", metadata: payload.metadata, task });
        }),
        Api.listen("task_progress", payload => {
            const task = tasks._tasks[payload.id];
            if (task === undefined)
                return;
            task.progress = payload.progress;
            tasks._notify(task, { event: "progress", progress: payload.progress, task });
        }),
        Api.listen("task_dependency", payload => {
            const task = tasks._tasks[payload.id];
            if (task === undefined)
                return;
            const dependency = payload.dependency;
            task.dependencies = task.dependencies.concat([dependency]);
            if (task.metadata.kind === "Aggregate") {
                tasks.registerTaskListener(dependency, function handler(event) {
                    if (event.event === "created") {
                        if (event.metadata.progress_unit !== task.metadata.progress_unit) {
                            // Units don't match. We don't want to handle this.
                            tasks.unregisterTaskListener(dependency, handler);
                        }
                    } else if (event.event === "progress") {
                        let completed = 0;
                        let total = 0;
                        for (const id of task.dependencies) {
                            const subTask = tasks._tasks[id];
                            if (subTask === undefined)
                                continue;
                            completed += subTask.progress.completed;
                            total += subTask.progress.total;
                        }
                        task.progress = { completed, total };
                    }
                });
            }
            tasks._notify(task, { event: "dependency", dependency, task });
        }),
        Api.listen("task_dropped", payload => {
            const task = tasks._tasks[payload.id];
            if (task === undefined)
                return;
            task.status = payload.status;
            tasks._notify(task, { event: "dropped", status: payload.status, task });
            task.listeners = [];
        })
    ]
}
