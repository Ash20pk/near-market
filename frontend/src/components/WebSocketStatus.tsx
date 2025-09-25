'use client';

import React from 'react';
import { useGlobalWebSocket } from '../hooks/useGlobalWebSocket';

export function WebSocketStatus() {
  const { isConnected, error, connectionStatus } = useGlobalWebSocket();

  return (
    <div className="fixed bottom-4 right-4 z-50 bg-gray-800 text-white p-3 rounded-lg shadow-lg text-xs">
      <div className="flex items-center gap-2">
        <div className={`w-2 h-2 rounded-full ${isConnected ? 'bg-green-500' : 'bg-red-500'}`} />
        <span>WebSocket: {isConnected ? 'Connected' : 'Disconnected'}</span>
      </div>
      {error && (
        <div className="text-red-400 mt-1">{error}</div>
      )}
      <div className="text-gray-400 mt-1">
        Subscribers: {connectionStatus.subscriberCount}
      </div>
    </div>
  );
}