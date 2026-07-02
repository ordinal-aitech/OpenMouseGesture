import { useEffect, useState } from 'react';
import { getVersion, getIconBytes } from '../../api/commands';
import './InfoTab.css';

export function InfoTab() {
  const [version, setVersion] = useState<string>('');
  const [iconUrl, setIconUrl] = useState<string>('');

  useEffect(() => {
    let objectUrl = '';

    const load = async () => {
      try {
        const [ver, bytes] = await Promise.all([getVersion(), getIconBytes()]);
        setVersion(ver);
        const blob = new Blob([new Uint8Array(bytes)], { type: 'image/png' });
        objectUrl = URL.createObjectURL(blob);
        setIconUrl(objectUrl);
      } catch (error) {
        console.error('Failed to load app info:', error);
        setVersion('Unknown');
      }
    };

    load();

    return () => {
      if (objectUrl) {
        URL.revokeObjectURL(objectUrl);
      }
    };
  }, []);

  return (
    <div className="info-tab">
      <div className="info-content">
        <div className="info-header">
          <img src={iconUrl} alt="GestureHotkeyApp Icon" className="info-icon" />
          <h1 className="info-title">GestureHotkeyApp</h1>
        </div>

        <div className="info-section">
          <div className="info-item">
            <span className="info-label">バージョン:</span>
            <span className="info-value">{version}</span>
          </div>

          <div className="info-item">
            <span className="info-label">ベース OSS:</span>
            <a
              href="https://github.com/7-rate/OpenMouseGesture/"
              className="info-link"
              target="_blank"
              rel="noopener noreferrer"
            >
              OpenMouseGesture
            </a>
          </div>
        </div>

        <div className="info-section">
          <h2 className="info-subtitle">アプリ概要</h2>
          <div className="info-disclaimer">
            <p>GestureHotkeyApp は OpenMouseGesture をベースに改造する自己使用向けアプリです。</p>
            <p>今後は 3 トリガー設定、trigger slot ごとの Hotkey 割り当て、軌跡色設定を追加していきます。</p>
          </div>
        </div>
      </div>
    </div>
  );
}
