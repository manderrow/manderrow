pragma Singleton
import QtQuick
import Manderrow

// Promises for the backend's calls, and async functions, which QML's JavaScript doesn't have.
QtObject {
    /// A promise of what a call returns: its value, or the objects it returns, which QML then
    /// owns. It rejects with the call's error, as `nativeError` makes it, or `abortedError`.
    function promise(call: Call): var {
        return new Promise((resolve, reject) => {
            function settle() {
                switch (call.status) {
                case Call.Running:
                    return;
                case Call.Succeeded:
                    resolve(_result(call));
                    break;
                case Call.Aborted:
                    reject(abortedError());
                    break;
                default:
                    reject(nativeError(call.errorMessages, call.backtrace));
                }
                call.statusChanged.disconnect(settle);
            }
            call.statusChanged.connect(settle);
            settle();
        });
    }

    function _result(call: Call): var {
        if (call.resultKind === Call.Value)
            return call.value;
        const objects = [];
        for (let i = 0; i < call.objectCount; i++)
            objects.push(call.takeObject(i));
        return call.resultKind === Call.List ? objects : (objects[0] ?? null);
    }

    /// Whether a value is a call, which `promise` waits for.
    function isCall(value: var): bool {
        return value instanceof Call;
    }

    /// A promise of a call, or of another value as `Promise.resolve` makes it.
    function resolve(value: var): var {
        return isCall(value) ? promise(value as Call) : Promise.resolve(value);
    }

    /// Runs a generator function as an async function: each `yield`ed promise or call is awaited,
    /// and its result sent back into the generator. Returns a promise of the generator's return
    /// value.
    function co(generator: var, thisArg: var): var {
        return new Promise((resolve, reject) => {
            const it = generator.call(thisArg);
            function step(method, arg) {
                let result;
                try {
                    result = it[method](arg);
                } catch (e) {
                    reject(e);
                    return;
                }
                if (result.done)
                    resolve(result.value);
                else
                    _resolve(result.value).then(v => step("next", v), e => step("throw", e));
            }
            step("next", undefined);
        });
    }

    // `resolve`, which `co`'s parameter of the same name shadows
    function _resolve(value: var): var {
        return resolve(value);
    }

    /// An error from the backend, with every message in its cause chain and a backtrace.
    function nativeError(messages: list<string>, backtrace: string): var {
        const e = new Error(messages[0]);
        e.isNative = true;
        e.messages = messages;
        e.backtrace = backtrace;
        e.toString = () => `NativeError: ${messages.join("\n")}`;
        return e;
    }

    /// The error of a call that the user cancelled.
    function abortedError(): var {
        const e = new Error("Aborted by the user");
        e.isAborted = true;
        return e;
    }
}
