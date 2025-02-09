import { invoke } from "@tauri-apps/api/core";
import { wrapInvoke } from "../api";

export type SafeOsString =
  | { Unicode: string }
  | { NonUnicodeBytes: number[] }
  | { NonUnicodeWide: number[] }
  | { NonUnicodeOther: string };

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
      Connect: {};
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
        level: "Error" | "Warn" | "Info" | "Debug" | "Trace";
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

export async function sendS2CMessage(msg: S2CMessage) {
  return await wrapInvoke<void>(() => invoke("send_s2c_message", { msg }));
}
