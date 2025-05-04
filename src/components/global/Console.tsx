import { For, Match, Show, Switch, createRenderEffect, createUniqueId, onMount, useContext } from "solid-js";

import { LOG_LEVELS, SafeOsString, sendS2CMessage } from "../../api/ipc";
import styles from "./Console.module.css";
import Dialog, { dialogStyles } from "./Dialog";
import { t } from "../../i18n/i18n";
import { ErrorContext } from "./ErrorBoundary";
import SelectDropdown from "./SelectDropdown";
import { createStore } from "solid-js/store";
import {
  connections,
  connectionsUpdate,
  doctorReports,
  focusedConnection,
  IdentifiedDoctorReport,
  setDoctorReports,
  setFocusedConnection,
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

  const [visibleLevels, setVisibleLevels] = createStore<{ [k in (typeof LOG_LEVELS)[number]]: boolean }>({
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
              focusedConnection().createdTime.toLocaleString()
            )}
          </div>
        </div>
      </header>
      <div class={styles.console} ref={consoleContainer} data-overflowed="false">
        <For each={focusedConnection()?.events()} fallback={<p>Game not running.</p>}>
          {(event) => {
            if ("Output" in event) {
              let line: string;
              if ("Unicode" in event.Output.line) {
                line = event.Output.line.Unicode;
              } else if ("Bytes" in event.Output.line) {
                line = JSON.stringify(event.Output.line.Bytes);
              } else {
                throw Error();
              }
              return (
                <p>
                  <span class={styles.event__type}>
                    <Switch>
                      <Match when={event.Output.channel === "Out"}>[OUT]</Match>
                      <Match when={event.Output.channel === "Err"}>[ERR]</Match>
                    </Switch>
                  </span>{" "}
                  {line}
                </p>
              );
            } else if ("Log" in event) {
              return (
                <Show when={visibleLevels[event.Log.level]}>
                  <p>
                    <span class={styles.event__type}>[{event.Log.level}]</span> <span>{event.Log.scope}</span>:{" "}
                    <span>{event.Log.message}</span>
                  </p>
                </Show>
              );
            } else if ("Connect" in event) {
              return (
                <p>
                  <span class={styles.event__type}>[CONNECT]</span> Agent connected to Manderrow from process{" "}
                  {event.Connect.pid}
                </p>
              );
            } else if ("Start" in event) {
              return (
                <p>
                  <span class={styles.event__type}>[START]</span>{" "}
                  <For each={Object.entries(event.Start.env)}>
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
                  <DisplaySafeOsString string={event.Start.command} />{" "}
                  <For each={event.Start.args}>
                    {(arg) => (
                      <>
                        {" "}
                        <DisplaySafeOsString string={arg} />
                      </>
                    )}
                  </For>
                </p>
              );
            } else if ("Exit" in event) {
              return (
                <p>
                  <span class={styles.event__type}>[EXIT]</span>{" "}
                  <Show when={event.Exit.code} fallback="Unknown exit code">
                    <span>{event.Exit.code}</span>
                  </Show>
                </p>
              );
            } else if ("Crash" in event) {
              return (
                <p>
                  <span class={styles.event__type}>[CRASH]</span> <span>{event.Crash.error}</span>
                </p>
              );
            } else if ("Error" in event) {
              return (
                <p>
                  <span class={styles.event__type}>[ERROR]</span> <span>{(event.Error.error as any).toString()}</span>
                </p>
              );
            }
          }}
        </For>
      </div>
    </>
  );
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

  const report = () => props.report.DoctorReport;

  return (
    <Dialog>
      <div class={dialogStyles.dialog__container}>
        <h2 class={dialogStyles.dialog__title}>Uh oh!</h2>
        <p class={styles.dialog__message}>
          {translateUnchecked(report().message ?? `doctor.${report().translation_key}.message`, report().message_args)}
        </p>

        <ul>
          <For each={report().fixes}>
            {(fix) => (
              <li>
                <div>{translateUnchecked(`doctor.${report().translation_key}.fixes.${fix.id}.label`, fix.label)}</div>
                <div>
                  {translateUnchecked(
                    `doctor.${report().translation_key}.fixes.${fix.id}.description`,
                    fix.description,
                  )}
                </div>
                <button
                  on:click={async () => {
                    try {
                      await sendS2CMessage(props.report.connId, {
                        PatientResponse: {
                          id: report().id,
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
                    `doctor.${report().translation_key}.fixes.${fix.id}.confirm_label`,
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
