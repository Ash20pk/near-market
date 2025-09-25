/**
 * Hook for using the global WebSocket connection
 * Provides persistent WebSocket connection across page navigation
 */

import { useState, useEffect, useRef } from 'react';
import { globalWebSocketManager } from '../services/globalWebSocket';
import {
  OrderbookUpdateMessage,
  TradeExecutedMessage,
  OrderUpdateMessage
} from '../services/websocket';

interface UseGlobalWebSocketOptions {
  marketId?: string;
  outcome?: number;
  onOrderbookUpdate?: (message: OrderbookUpdateMessage) => void;
  onTradeExecuted?: (message: TradeExecutedMessage) => void;
  onOrderUpdate?: (message: OrderUpdateMessage) => void;
}

export function useGlobalWebSocket(options: UseGlobalWebSocketOptions = {}) {
  const [isConnected, setIsConnected] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const componentId = useRef(`ws-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`);

  useEffect(() => {
    const { marketId, outcome, onOrderbookUpdate, onTradeExecuted, onOrderUpdate } = options;

    // Subscribe to global WebSocket with filtered handlers
    const unsubscribe = globalWebSocketManager.subscribe(componentId.current, {
      onConnect: () => {
        setIsConnected(true);
        setError(null);
      },
      onDisconnect: () => {
        setIsConnected(false);
      },
      onError: () => {
        setError('WebSocket connection failed');
        setIsConnected(false);
      },
      onOrderbookUpdate: (message: OrderbookUpdateMessage) => {
        // Filter by market/outcome if specified
        if (!marketId || !outcome ||
            (message.market_id === marketId && message.outcome === outcome)) {
          onOrderbookUpdate?.(message);
        }
      },
      onTradeExecuted: (message: TradeExecutedMessage) => {
        // Filter by market if specified
        if (!marketId || message.trade.market_id === marketId) {
          onTradeExecuted?.(message);
        }
      },
      onOrderUpdate: (message: OrderUpdateMessage) => {
        onOrderUpdate?.(message);
      },
    });

    // Cleanup on unmount
    return unsubscribe;
  }, [options.marketId, options.outcome]); // Re-subscribe if market/outcome changes

  return {
    isConnected,
    error,
    connectionStatus: globalWebSocketManager.getConnectionStatus(),
  };
}