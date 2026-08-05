; WBMusic NSIS installer hooks
; 配置入口: bundle.windows.nsis.installerHooks
;
; 说明:
; - NSIS_HOOK_POSTUNINSTALL 在卸载器删除文件/注册表/快捷方式之后运行
; - 这里清理应用数据目录，实现"卸载干净"：
;   * $APPDATA\com.wbmusic.app      -> session.json（登录态）
;   * $LOCALAPPDATA\com.wbmusic.app -> WebView2 浏览数据（EBWebView 缓存、Cookie 等）
;
; 已知残留（无法通过安装器清除）:
; - Windows 凭据管理器中的 "wbmusic" 条目（登录 Cookie，由应用 keyring 写入，
;   NSIS 脚本无法操作凭据管理器）。如需彻底清除，请在应用内"退出登录"，
;   或在 控制面板 -> 凭据管理器 中手动删除。

!macro NSIS_HOOK_POSTUNINSTALL
  ; 清理应用数据（登录态 session.json + WebView2 缓存）
  RMDir /r "$APPDATA\com.wbmusic.app"
  RMDir /r "$LOCALAPPDATA\com.wbmusic.app"
!macroend
