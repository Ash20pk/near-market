/**
 * Global WebSocket Service
 * Maintains a single persistent WebSocket connection across page navigation
 */

import {
  OrderbookWebSocketService,
  createOrderbookWebSocket,
  OrderbookUpdateMessage,
  TradeExecutedMessage,
  OrderUpdateMessage
} from './websocket';

class GlobalWebSocketManager {
  private wsService: OrderbookWebSocketService | null = null;
  private isConnected = false;
  private subscribers = new Set<{
    id: string;
    onOrderbookUpdate?: (message: OrderbookUpdateMessage) => void;
    onTradeExecuted?: (message: TradeExecutedMessage) => void;
    onOrderUpdate?: (message: OrderUpdateMessage) => void;
    onConnect?: () => void;
    onDisconnect?: () => void;
    onError?: (error: Event) => void;
  }>();
  private connectionPromise: Promise<void> | null = null;

  constructor() {
    // Auto-cleanup on page unload
    if (typeof window !== 'undefined') {
      window.addEventListener('beforeunload', () => {
        this.disconnect();
      });
    }
  }

  async ensureConnected(): Promise<void> {
    // If already connected or connecting, return existing promise
    if (this.isConnected || this.connectionPromise) {
      return this.connectionPromise || Promise.resolve();
    }

    // Start new connection
    this.connectionPromise = this.connect();
    return this.connectionPromise;
  }

  private async connect(): Promise<void> {
    console.log('[GlobalWebSocket] Establishing persistent WebSocket connection...');

    if (this.wsService) {
      console.log('[GlobalWebSocket] Cleaning up existing connection...');
      this.wsService.disconnect();
    }

    this.wsService = createOrderbookWebSocket();

    try {
      await this.wsService.connect({
        onConnect: () => {
          console.log('[GlobalWebSocket] ✅ Global WebSocket connected');
          this.isConnected = true;
          this.connectionPromise = null;

          // Notify all subscribers
          this.subscribers.forEach(subscriber => {
            subscriber.onConnect?.();
          });
        },
        onDisconnect: () => {
          console.log('[GlobalWebSocket] 🔌 Global WebSocket disconnected');
          this.isConnected = false;
          this.connectionPromise = null;

          // Notify all subscribers
          this.subscribers.forEach(subscriber => {
            subscriber.onDisconnect?.();
          });
        },
        onError: (error) => {
          console.error('[GlobalWebSocket] ❌ Global WebSocket error:', error);
          this.isConnected = false;
          this.connectionPromise = null;

          // Notify all subscribers
          this.subscribers.forEach(subscriber => {
            subscriber.onError?.(error);
          });
        },
        onOrderbookUpdate: (message: OrderbookUpdateMessage) => {
          // Broadcast to all interested subscribers
          this.subscribers.forEach(subscriber => {
            subscriber.onOrderbookUpdate?.(message);
          });
        },
        onTradeExecuted: (message: TradeExecutedMessage) => {
          // Broadcast to all interested subscribers
          this.subscribers.forEach(subscriber => {
            subscriber.onTradeExecuted?.(message);
          });
        },
        onOrderUpdate: (message: OrderUpdateMessage) => {
          // Broadcast to all interested subscribers
          this.subscribers.forEach(subscriber => {
            subscriber.onOrderUpdate?.(message);
          });
        },
      });
    } catch (error) {
      console.error('[GlobalWebSocket] Failed to establish connection:', error);
      this.isConnected = false;
      this.connectionPromise = null;
      throw error;
    }
  }

  subscribe(id: string, handlers: {
    onOrderbookUpdate?: (message: OrderbookUpdateMessage) => void;
    onTradeExecuted?: (message: TradeExecutedMessage) => void;
    onOrderUpdate?: (message: OrderUpdateMessage) => void;
    onConnect?: () => void;
    onDisconnect?: () => void;
    onError?: (error: Event) => void;
  }) {
    console.log(`[GlobalWebSocket] Subscribing component: ${id}`);

    const subscriber = { id, ...handlers };
    this.subscribers.add(subscriber);

    // Auto-connect when first subscriber joins
    if (this.subscribers.size === 1) {
      this.ensureConnected().catch(error => {
        console.error('[GlobalWebSocket] Failed to auto-connect:', error);
      });
    }

    // If already connected, immediately notify
    if (this.isConnected) {
      handlers.onConnect?.();
    }

    // Return unsubscribe function
    return () => {
      console.log(`[GlobalWebSocket] Unsubscribing component: ${id}`);
      this.subscribers.delete(subscriber);

      // Disconnect when no subscribers left
      if (this.subscribers.size === 0) {
        console.log('[GlobalWebSocket] No subscribers left, disconnecting...');
        setTimeout(() => {
          // Delay disconnect to avoid rapid connect/disconnect cycles during navigation
          if (this.subscribers.size === 0) {
            this.disconnect();
          }
        }, 5000); // Wait 5 seconds before disconnecting
      }
    };
  }

  disconnect() {
    if (this.wsService) {
      console.log('[GlobalWebSocket] Disconnecting global WebSocket...');
      this.wsService.disconnect();
      this.wsService = null;
    }
    this.isConnected = false;
    this.connectionPromise = null;
    this.subscribers.clear();
  }

  getConnectionStatus() {
    return {
      isConnected: this.isConnected,
      subscriberCount: this.subscribers.size,
    };
  }
}

// Global singleton instance
export const globalWebSocketManager = new GlobalWebSocketManager();