import * as api from "./api/commands";
import { useStore } from "./store/useStore";

export async function handleApiError(error: unknown) {
  const errorMessage = error instanceof Error ? error.message : String(error);
  
  // 検証エラーを含むかチェック
  if (errorMessage.includes("検証エラー") || errorMessage.includes("validation")) {
    const setValidationError = useStore.getState().setValidationError;
    
    // config.jsonのエラーかチェック
    if (errorMessage.includes("config.json")) {
      const [configPath, validationError] = await Promise.all([
        api.getConfigFilePath(),
        api.getConfigValidationError(),
      ]);
      setValidationError({
        fileType: "config",
        filePath: configPath,
        errorMessage: validationError || errorMessage,
      });
      return true;
    }
    
    // gestures.jsonのエラーかチェック
    if (errorMessage.includes("gestures.json")) {
      const [gesturesPath, validationError] = await Promise.all([
        api.getGesturesFilePath(),
        api.getGesturesValidationError(),
      ]);
      setValidationError({
        fileType: "gestures",
        filePath: gesturesPath,
        errorMessage: validationError || errorMessage,
      });
      return true;
    }
  }
  
  return false;
}
