import { invoke } from "@tauri-apps/api/core";
import { wrapInvoke } from "../api";

export type SafeOsString = { Unicode: string } | { NonUnicodeBytes: number[] } | { NonUnicodeWide: number[] };

export interface DoctorReport {
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

export type C2SMessage =
  | {
      Connect: {
        pid: number;
      };
    }
  | {
      Disconnect: {};
    }
  | {
      Start: {
        command: SafeOsString;
        args: SafeOsString[];
        env: { [key: string]: SafeOsString };
      };
    }
  | {
      Log: {
        level: "Critical" | "Error" | "Warn" | "Info" | "Debug" | "Trace";
        scope: string;
        message: string;
      };
    }
  | {
      Output: {
        channel: "Out" | "Err";
        line:
          | {
              Unicode: string;
            }
          | {
              Bytes: number[];
            };
      };
    }
  | {
      Exit: {
        code?: number;
      };
    }
  | {
      Crash: {
        error: string;
      };
    }
  | {
      DoctorReport: DoctorReport;
    };

export type S2CMessage = {
  PatientResponse: {
    id: string;
    choice: string;
  };
};

export async function allocateIpcConnection(): Promise<number> {
  return await wrapInvoke(() => invoke("allocate_ipc_connection", {}));
}

export async function sendS2CMessage(connId: number, msg: S2CMessage): Promise<void> {
  return await wrapInvoke<void>(() => invoke("send_s2c_message", { connId, msg }));
}

export async function killIpcClient(connId: number): Promise<void> {
  return await wrapInvoke<void>(() => invoke("kill_ipc_client", { connId }));
}
