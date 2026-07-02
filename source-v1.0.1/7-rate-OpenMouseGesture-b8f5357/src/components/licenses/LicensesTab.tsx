/**
 * ライセンス表示タブ
 * 
 * 本ソフトウェアのライセンスとサードパーティライブラリのライセンス情報を表示する。
 */

import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './LicensesTab.css';

export function LicensesTab() {
  const [licenseHtml, setLicenseHtml] = useState<string>('');

  useEffect(() => {
    loadLicenseInfo();
  }, []);

  const loadLicenseInfo = async () => {
    try {
      const html = await invoke<string>('get_license_info');
      setLicenseHtml(html);
    } catch (error) {
      console.error('Failed to load license info:', error);
    }
  };

  if (!licenseHtml) {
    return <div className="licenses-tab loading">Loading license information...</div>;
  }

  return (
    <div className="licenses-tab">
      <div 
        className="license-content"
        dangerouslySetInnerHTML={{ __html: licenseHtml }}
      />
    </div>
  );
}
