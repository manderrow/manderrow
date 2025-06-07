import {
  For,
  Match,
  Show,
  Switch,
  createRenderEffect,
  createSignal,
  createUniqueId,
  onMount,
  useContext,
} from "solid-js";
import { createStore } from "solid-js/store";

import { LOG_LEVELS, SafeOsString, sendS2CMessage } from "../../api/ipc";
import { bindValue } from "../global/Directives";
import styles from "./Console.module.css";
import Dialog, { dialogStyles } from "./Dialog";
import { t } from "../../i18n/i18n";
import { ErrorContext } from "./ErrorBoundary";
import SelectDropdown from "./SelectDropdown";
import {
  connections,
  connectionsUpdate,
  doctorReports,
  focusedConnection,
  IdentifiedDoctorReport,
  setDoctorReports,
  setFocusedConnection,
  type Event,
} from "../../console";

const translateUnchecked = t as (key: string, args: Object | undefined) => string;

export function DoctorReports() {
  return (
    <For each={doctorReports()}>
      {(report, i) => (
        <DoctorDialog
          report={report}
          onDismiss={() => {
            setDoctorReports((reports) => {
              return [...reports.slice(0, i()), ...reports.slice(i() + 1)];
            });
          }}
        />
      )}
    </For>
  );
}

type VisibleLevels = { [k in (typeof LOG_LEVELS)[number]]: boolean };

export default function Console() {
  function getSelectConnectionOptions() {
    // track updates
    connectionsUpdate();
    return Object.fromEntries(
      Array.from(connections.keys()).map((id) => [
        id.toString(),
        { value: id, selected: id === focusedConnection()?.id },
      ]),
    );
  }

  const [visibleLevels, setVisibleLevels] = createStore<VisibleLevels>({
    CRITICAL: true,
    ERROR: true,
    WARN: true,
    INFO: true,
    DEBUG: false,
    TRACE: false,
  });

  let consoleContainer!: HTMLDivElement;

  onMount(() => {
    createRenderEffect(() => {
      focusedConnection()?.events().length;

      // If no overflow yet, don't try to set overflowed to true by returning here
      if (
        consoleContainer.scrollHeight === consoleContainer.clientHeight &&
        consoleContainer.dataset.overflowed === "false"
      )
        return;

      // If at bottom of scroll, scroll to the new bottom position after DOM updates,
      // and set overflowed to true to stop always scrolling to bottom
      if (
        consoleContainer.scrollHeight - consoleContainer.clientHeight <= consoleContainer.scrollTop + 1 ||
        consoleContainer.dataset.overflowed === "false"
      ) {
        queueMicrotask(() => {
          consoleContainer.scrollTop = consoleContainer.scrollHeight - consoleContainer.clientHeight;
          consoleContainer.dataset.overflowed = "true";
        });
      }
    });
  });

  const [searchInput, setSearchInput] = createSignal("");

  return (
    <>
      <header class={styles.header}>
        <div class={styles.header__options}>
          <div class={styles.header__group}>
            <div class={styles.header__subgroup}>
              <p>View log:</p>
              <SelectDropdown
                label={{ labelText: "value" }}
                options={getSelectConnectionOptions()}
                onChanged={(id, selected) => {
                  if (selected) {
                    setFocusedConnection(connections.get(id));
                  }
                }}
              />
            </div>
            <div class={styles.header__subgroup}>
              <label for="line-wrap">Line wrap</label>
              <input type="checkbox" name="line-wrap" id="line-wrap" checked />
            </div>
            <div class={styles.header__subgroup}>
              <input
                type="text"
                name="log-search"
                id="log-search"
                placeholder="Search log..."
                class={styles.logSearch}
                use:bindValue={[searchInput, setSearchInput]}
              />
            </div>
          </div>
          <div class={styles.header__group}>
            <div classList={{ [styles.header__subgroup]: true, [styles.toggleList]: true }}>
              <For each={LOG_LEVELS}>
                {(level) => {
                  const id = createUniqueId();
                  return (
                    <>
                      <input
                        id={id}
                        type="checkbox"
                        checked={visibleLevels[level]}
                        on:change={(event) => setVisibleLevels(level, event.target.checked)}
                        style="display:none"
                      />
                      <label class={styles.scopeToggle} for={id}>
                        {level}
                      </label>
                    </>
                  );
                }}
              </For>
            </div>
            <div classList={{ [styles.header__subgroup]: true, [styles.toggleList]: true }}>
              Scope toggles go here in future
            </div>
          </div>
        </div>
        <div class={styles.header__group}>
          <div>
            <p class={styles.header__liveLogText}>
              {focusedConnection()?.status() !== "disconnected" ? "Live log" : "Created at"}
            </p>
            {focusedConnection()?.status() !== "disconnected" ? (
              <span class={styles.statusIndicator} data-connected={focusedConnection()?.status() === "connected"}>
                {focusedConnection()?.status() === "connected" ? "Connected" : "Disconnected"}
              </span>
            ) : (
              focusedConnection()!.createdTime.toLocaleString()
            )}
          </div>
        </div>
      </header>
      <div class={styles.console} ref={consoleContainer} data-overflowed="false">
        <For each={focusedConnection()?.events()} fallback={<p>Game not running.</p>}>
          {(event) => ConsoleEvent(event, visibleLevels, searchInput)}
        </For>
      </div>
    </>
  );
}

const STYLE_DISPLAY_NONE = { display: "none" };

function ConsoleEvent(event: Event, visibleLevels: VisibleLevels, searchInput: () => string) {
  const visible = () => {
    switch (event.type) {
      case "Log":
        return visibleLevels[event.level] && event.message.includes(searchInput());
      case "Output":
        return !("Unicode" in event.line) || event.line.Unicode.includes(searchInput());
      case "Connect":
      case "Disconnect":
      case "Start":
      case "Started":
      case "Exit":
      case "Crash":
      case "DoctorReport":
      case "Error":
        return true;
    }
  };

  const displayStyle = () => (visible() ? undefined : STYLE_DISPLAY_NONE);

  switch (event.type) {
    case "Output":
      let line: string;
      if ("Unicode" in event.line) {
        line = event.line.Unicode;
      } else if ("Bytes" in event.line) {
        line = JSON.stringify(event.line.Bytes);
      } else {
        throw Error();
      }
      return (
        <>
          <span class={styles.event__type} style={displayStyle()}>
            <Switch>
              <Match when={event.channel === "Out"}>OUT</Match>
              <Match when={event.channel === "Err"}>ERR</Match>
            </Switch>
          </span>
          <span class={styles.event__scope} style={displayStyle()}></span>
          <span class={styles.event__message} style={displayStyle()}>
            {line}
          </span>
        </>
      );
    case "Log":
      return (
        <>
          <span class={styles.event__type} style={displayStyle()} data-type={event.level}>
            {event.level}
          </span>
          <span class={styles.event__scope} style={displayStyle()}>
            <span>{event.scope}</span>:
          </span>
          <span class={styles.event__message} style={displayStyle()}>
            {event.message}
          </span>
        </>
      );
    case "Connect":
      return (
        <>
          <span class={styles.event__type} style={displayStyle()} data-type="CONNECT">
            CONNECT
          </span>
          <span class={styles.event__scope} style={displayStyle()}></span>
          <span class={styles.event__message} style={displayStyle()}>
            Game connected to Manderrow
          </span>
        </>
      );
    case "Start":
      return (
        <>
          <span class={styles.event__type} style={displayStyle()}>
            START
          </span>
          <span class={styles.event__scope} style={displayStyle()}></span>
          <span class={styles.event__message} style={displayStyle()}>
            <For each={Object.entries(event.env)}>
              {([k, v]) => {
                if ("Unicode" in v) {
                  return (
                    <>
                      {k}={JSON.stringify(v.Unicode)}{" "}
                    </>
                  );
                } else {
                  return (
                    <>
                      {k}={JSON.stringify(v)}{" "}
                    </>
                  );
                }
              }}
            </For>
            <DisplaySafeOsString string={event.command} />{" "}
            <For each={event.args}>
              {(arg) => (
                <>
                  {" "}
                  <DisplaySafeOsString string={arg} />
                </>
              )}
            </For>
          </span>
        </>
      );
    case "Started":
      return (
        <>
          <span class={styles.event__type} style={displayStyle()} data-type="STARTED">
            STARTED
          </span>
          <span class={styles.event__scope} style={displayStyle()}></span>
          <span class={styles.event__message} style={displayStyle()}>
            Game process started with id <span>{event.pid}</span>
          </span>
        </>
      );
    case "Exit":
      return (
        <>
          <span class={styles.event__type} style={displayStyle()}>
            EXIT
          </span>
          <span class={styles.event__scope} style={displayStyle()}></span>
          <span class={styles.event__message} style={displayStyle()}>
            <Show when={event.code} fallback="Unknown exit code">
              <span>{event.code}</span>
            </Show>
          </span>
        </>
      );
    case "Crash":
      return (
        <>
          <span class={styles.event__type} style={displayStyle()} data-type="CRASH">
            CRASH
          </span>
          <span class={styles.event__scope} style={displayStyle()}></span>
          <span class={styles.event__message} style={displayStyle()}>
            {event.error}
          </span>
        </>
      );
    case "Error":
      return (
        <>
          <span class={styles.event__type} style={displayStyle()} data-type="ERROR">
            ERROR
          </span>
          <span class={styles.event__scope} style={displayStyle()}></span>
          <span class={styles.event__message} style={displayStyle()}>
            {(event.error as any).toString()}
          </span>
        </>
      );
    case "Disconnect":
      return (
        <>
          <span class={styles.event__type} style={displayStyle()}>
            DISCONNECT
          </span>
          <span class={styles.event__scope} style={displayStyle()}></span>
          <span class={styles.event__message} style={displayStyle()}>
            Game disconnected from Manderrow
          </span>
        </>
      );
    case "DoctorReport":
      return <></>;
  }
}

function DisplaySafeOsString(props: { string: SafeOsString }) {
  const s = props.string;
  return (
    <Switch>
      <Match when={"Unicode" in s ? s.Unicode : null}>{(s) => JSON.stringify(s())}</Match>
      <Match when={"NonUnicodeBytes" in s ? s.NonUnicodeBytes : null}>{(b) => JSON.stringify(b())}</Match>
      <Match when={"NonUnicodeWide" in s ? s.NonUnicodeWide : null}>{(b) => JSON.stringify(b())}</Match>
    </Switch>
  );
}

function DoctorDialog(props: { report: IdentifiedDoctorReport; onDismiss: () => void }) {
  const reportErr = useContext(ErrorContext)!;

  return (
    <Dialog>
      <div class={dialogStyles.dialog__container}>
        <h2 class={dialogStyles.dialog__title}>Uh oh!</h2>
        <p class={styles.dialog__message}>
          {translateUnchecked(
            props.report.message ?? `doctor.${props.report.translation_key}.message`,
            props.report.message_args,
          )}
        </p>

        <ul>
          <For each={props.report.fixes}>
            {(fix) => (
              <li>
                <div>
                  {translateUnchecked(`doctor.${props.report.translation_key}.fixes.${fix.id}.label`, fix.label)}
                </div>
                <div>
                  {translateUnchecked(
                    `doctor.${props.report.translation_key}.fixes.${fix.id}.description`,
                    fix.description,
                  )}
                </div>
                <button
                  on:click={async () => {
                    try {
                      await sendS2CMessage(props.report.connId, {
                        PatientResponse: {
                          id: props.report.id,
                          choice: fix.id,
                        },
                      });
                    } catch (e) {
                      reportErr(e);
                    } finally {
                      props.onDismiss();
                    }
                  }}
                >
                  {translateUnchecked(
                    `doctor.${props.report.translation_key}.fixes.${fix.id}.confirm_label`,
                    fix.confirm_label,
                  )}
                </button>
              </li>
            )}
          </For>
        </ul>
      </div>
    </Dialog>
  );
}
