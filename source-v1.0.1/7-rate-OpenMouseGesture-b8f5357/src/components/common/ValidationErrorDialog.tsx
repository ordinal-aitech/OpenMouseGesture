import { useState } from "react";
import { Dialog } from "./Dialog";
import { Button } from "./Button";
import { openPath } from "@tauri-apps/plugin-opener";
import { message as showMessage } from "@tauri-apps/plugin-dialog";
import * as api from "../../api/commands";
import "./ValidationErrorDialog.css";

interface ValidationErrorDialogProps {
  fileType: "config" | "gestures";
  filePath: string;
  errorMessage: string;
  onResolved: () => void;
}

export function ValidationErrorDialog({
  fileType,
  filePath,
  errorMessage,
  onResolved,
}: ValidationErrorDialogProps) {
  const [isProcessing, setIsProcessing] = useState(false);

  const fileName = fileType === "config" ? "config.json" : "gestures.json";
  const title = `${fileName} 検証エラー`;
  const dialogMessage = `${fileName}の書式が不正です。\n\nエディタで開いて修正するか、デフォルト設定で上書きしてください。`;

  const handleOpenEditor = async () => {
    console.log("[ValidationErrorDialog] Opening editor for:", filePath);
    try {
      setIsProcessing(true);
      await openPath(filePath);
      console.log("[ValidationErrorDialog] Editor opened, closing app");
      // エディタで開いた後、アプリを終了
      setTimeout(() => {
        window.close();
      }, 500);
    } catch (err) {
      console.error("[ValidationErrorDialog] Failed to open file:", err);
      await showMessage(`ファイルを開けませんでした: ${err}`, {
        title: "エラー",
        kind: "error",
      });
      setIsProcessing(false);
    }
  };

  const handleResetDefault = async () => {
    console.log("[ValidationErrorDialog] Resetting to default:", fileType);
    try {
      setIsProcessing(true);
      if (fileType === "config") {
        await api.resetConfigToDefault();
        console.log("[ValidationErrorDialog] Config reset successful");
      } else {
        await api.resetGesturesToDefault();
        console.log("[ValidationErrorDialog] Gestures reset successful");
      }
      onResolved();
    } catch (err) {
      console.error("[ValidationErrorDialog] Failed to reset to default:", err);
      await showMessage(`デフォルト設定の読み込みに失敗しました: ${err}`, {
        title: "エラー",
        kind: "error",
      });
      setIsProcessing(false);
    }
  };

  return (
    <Dialog
      open={true}
      title={title}
      onClose={() => {}}
      showCloseButton={false}
    >
      <div className="validation-error-content">
        <div className="validation-error-message">
          <p>{dialogMessage}</p>
          <div className="validation-error-path">
            <strong>ファイルパス:</strong>
            <code>{filePath}</code>
          </div>
        </div>

        <div className="validation-error-details">
          <strong>エラー内容:</strong>
          <pre>{errorMessage}</pre>
        </div>

        <div className="validation-error-actions">
          <Button
            onClick={handleOpenEditor}
            disabled={isProcessing}
            variant="primary"
          >
            エディタで開く
          </Button>
          <Button
            onClick={handleResetDefault}
            disabled={isProcessing}
            variant="secondary"
          >
            デフォルトで上書き
          </Button>
        </div>
      </div>
    </Dialog>
  );
}
