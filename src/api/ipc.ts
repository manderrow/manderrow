import { invoke } from "@tauri-apps/api/core";
import { wrapInvoke } from "./api";

export type SafeOsString = { Unicode: string } | { NonUnicodeBytes: number[] } | { NonUnicodeWide: number[] };

export interface DoctorReport {
  type: "DoctorReport";
  id: string;
  translation_key: string;
  message?: string;
  message_args?: Object;
  fixes: DoctorFix[];
}

export interface DoctorFix {
  id: string;
  label?: Object;
  confirm_label?: Object;
  description?: Object;
}

export const LOG_LEVELS = ["CRITICAL", "ERROR", "WARN", "INFO", "DEBUG", "TRACE"] as const;

export type C2SMessage =
  | {
      type: "Connect";
    }
  | {
      type: "Disconnect";
    }
  | {
      type: "Start";
      command: SafeOsString;
      args: SafeOsString[];
      env: { [key: string]: SafeOsString };
    }
  | {
      type: "Started";
      pid: number;
    }
  | {
      type: "Log";
      level: (typeof LOG_LEVELS)[number];
      scope: string;
      message: string;
    }
  | {
      type: "Output";
      channel: "Out" | "Err";
      line:
        | {
            Unicode: string;
          }
        | {
            Bytes: number[];
          };
    }
  | {
      type: "Exit";
      code?: number;
    }
  | {
      type: "Crash";
      error: string;
    }
  | DoctorReport;

export type S2CMessage = {
  type: "PatientResponse";
  id: string;
  choice: string;
};

export async function allocateIpcConnection(): Promise<number> {
  return await wrapInvoke(() => invoke("allocate_ipc_connection", {}));
}

export async function sendS2CMessage(connId: number, msg: S2CMessage): Promise<void> {
  return await wrapInvoke(() => invoke("send_s2c_message", { connId, msg }));
}

export async function killIpcClient(connId: number): Promise<void> {
  return await wrapInvoke(() => invoke("kill_ipc_client", { connId }));
}

export async function getIpcConnections(): Promise<number[]> {
  return await wrapInvoke(() => invoke("get_ipc_connections"));
}
