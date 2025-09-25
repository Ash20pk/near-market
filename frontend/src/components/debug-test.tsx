'use client';

import { useState } from 'react';
import { useWallet } from './near-wallet';

export function DebugTest() {
  const [result, setResult] = useState<string>('');
  const [loading, setLoading] = useState(false);
  const { nearService } = useWallet();

  const testMarkets = async () => {
    console.log('[DebugTest] Testing market loading...');
    setLoading(true);
    setResult('Testing...');

    try {
      const markets = await nearService.getMarkets();
      console.log('[DebugTest] Got markets:', markets);
      setResult(`✅ Success! Found ${markets.length} markets`);
    } catch (error) {
      console.error('[DebugTest] Error:', error);
      setResult(`❌ Error: ${error instanceof Error ? error.message : String(error)}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="fixed bottom-4 right-4 z-50 p-4 bg-gray-800 rounded-lg border border-gray-600 max-w-sm">
      <h3 className="text-white font-semibold mb-2">🧪 Debug Test</h3>
      <button
        onClick={testMarkets}
        disabled={loading}
        className="bg-blue-600 hover:bg-blue-700 disabled:bg-gray-600 text-white px-3 py-2 rounded text-sm mb-2"
      >
        {loading ? 'Testing...' : 'Test Market Loading'}
      </button>
      {result && (
        <div className="text-sm text-gray-300 mt-2 p-2 bg-gray-900 rounded">
          {result}
        </div>
      )}
    </div>
  );
}