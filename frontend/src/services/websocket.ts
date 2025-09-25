/**
 * WebSocket Service for Real-time Orderbook Updates
 * Provides reactive data streams for the frontend
 */

import { OrderbookSnapshot, PriceLevel, Trade } from './orderbook';

export interface WebSocketMessage {
  type: 'OrderbookUpdate' | 'TradeExecuted' | 'OrderUpdate';
}

export interface OrderbookUpdateMessage extends WebSocketMessage {
  type: 'OrderbookUpdate';
  market_id: string;
  outcome: number;
  snapshot: OrderbookSnapshot;
}

export interface TradeExecutedMessage extends WebSocketMessage {
  type: 'TradeExecuted';
  trade: Trade;
}

export interface OrderUpdateMessage extends WebSocketMessage {
  type: 'OrderUpdate';
  order_id: string;
  status: string;
  filled_size: number;
}

export interface ConnectionEstablishedMessage extends WebSocketMessage {
  type: 'connection_established';
  message: string;
  timestamp: string;
}

export type OrderbookWebSocketMessage = OrderbookUpdateMessage | TradeExecutedMessage | OrderUpdateMessage | ConnectionEstablishedMessage;

export interface WebSocketEventHandlers {
  onOrderbookUpdate?: (message: OrderbookUpdateMessage) => void;
  onTradeExecuted?: (message: TradeExecutedMessage) => void;
  onOrderUpdate?: (message: OrderUpdateMessage) => void;
  onConnect?: () => void;
  onDisconnect?: () => void;
  onError?: (error: Event) => void;
}

export class OrderbookWebSocketService {
  private ws: WebSocket | null = null;
  private url: string;
  private handlers: WebSocketEventHandlers = {};
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectDelay = 1000; // Start with 1 second
  private isConnecting = false;
  private shouldReconnect = true;

  constructor(url: string) {
    this.url = url;
  }

  /**
   * Connect to WebSocket and set up event handlers
   */
  connect(handlers: WebSocketEventHandlers = {}): Promise<void> {
    console.log('[WebSocket] Attempting to connect to:', this.url);
    this.handlers = handlers;
    this.shouldReconnect = true;

    return new Promise((resolve, reject) => {
      if (this.isConnecting || (this.ws && this.ws.readyState === WebSocket.CONNECTING)) {
        console.log('[WebSocket] Already connecting, skipping');
        return;
      }

      this.isConnecting = true;
      console.log('[WebSocket] Creating WebSocket connection...');

      try {
        this.ws = new WebSocket(this.url);
        console.log('[WebSocket] WebSocket object created, waiting for connection...');

        const connectTimeout = setTimeout(() => {
          console.error('[WebSocket] Connection timeout after 10 seconds');
          if (this.ws) {
            this.ws.close();
            reject(new Error('WebSocket connection timeout'));
          }
        }, 10000);

        this.ws.onopen = () => {
          console.log('[WebSocket] ✅ Connected successfully to orderbook WebSocket');
          clearTimeout(connectTimeout);
          this.isConnecting = false;
          this.reconnectAttempts = 0;
          this.reconnectDelay = 1000;

          this.handlers.onConnect?.();
          resolve();
        };

        this.ws.onmessage = (event) => {
          console.log('[WebSocket] 📨 Received message:', event.data);
          try {
            const message: OrderbookWebSocketMessage = JSON.parse(event.data);
            console.log('[WebSocket] Parsed message type:', message.type);
            this.handleMessage(message);
          } catch (error) {
            console.error('[WebSocket] ❌ Failed to parse WebSocket message:', error);
            console.error('[WebSocket] Raw message data:', event.data);
          }
        };

        this.ws.onclose = (event) => {
          console.log('[WebSocket] 🔌 Connection closed. Code:', event.code, 'Reason:', event.reason || 'No reason');
          console.log('[WebSocket] Was clean close:', event.wasClean);
          this.isConnecting = false;
          this.handlers.onDisconnect?.();

          if (this.shouldReconnect && this.reconnectAttempts < this.maxReconnectAttempts) {
            console.log('[WebSocket] 🔄 Scheduling reconnect attempt', this.reconnectAttempts + 1);
            this.scheduleReconnect();
          } else if (this.reconnectAttempts >= this.maxReconnectAttempts) {
            console.error('[WebSocket] ❌ Max reconnect attempts reached, giving up');
          }
        };

        this.ws.onerror = (error) => {
          console.error('[WebSocket] ❌ Connection error occurred:', error);
          console.error('[WebSocket] Error type:', error.type);
          this.isConnecting = false;
          this.handlers.onError?.(error);
          clearTimeout(connectTimeout);
          reject(error);
        };

      } catch (error) {
        this.isConnecting = false;
        reject(error);
      }
    });
  }

  /**
   * Handle incoming WebSocket messages
   */
  private handleMessage(message: OrderbookWebSocketMessage): void {
    console.log('[WebSocket] 🔄 Handling message type:', message.type);

    switch (message.type) {
      case 'OrderbookUpdate':
        console.log('[WebSocket] Processing orderbook update for market:', (message as OrderbookUpdateMessage).market_id);
        // Convert prices and sizes from orderbook format to frontend format
        const convertedMessage: OrderbookUpdateMessage = {
          ...message,
          snapshot: {
            ...message.snapshot,
            bids: message.snapshot.bids.map(level => ({
              ...level,
              price: level.price / 1000, // Convert to frontend format
              size: level.size / 1_000_000, // Convert to frontend format
            })),
            asks: message.snapshot.asks.map(level => ({
              ...level,
              price: level.price / 1000, // Convert to frontend format
              size: level.size / 1_000_000, // Convert to frontend format
            })),
          },
        };
        this.handlers.onOrderbookUpdate?.(convertedMessage);
        break;

      case 'TradeExecuted':
        // Convert trade data from orderbook format
        const convertedTrade: TradeExecutedMessage = {
          ...message,
          trade: {
            ...message.trade,
            price: message.trade.price / 1000, // Convert to frontend format
            size: message.trade.size / 1_000_000, // Convert to frontend format
          },
        };
        this.handlers.onTradeExecuted?.(convertedTrade);
        break;

      case 'OrderUpdate':
        // Convert filled size from orderbook format
        const convertedOrderUpdate: OrderUpdateMessage = {
          ...message,
          filled_size: message.filled_size / 1_000_000, // Convert to frontend format
        };
        this.handlers.onOrderUpdate?.(convertedOrderUpdate);
        break;

      case 'connection_established':
        console.log('[WebSocket] 🎉 Connection established:', message.message);
        console.log('[WebSocket] Server timestamp:', message.timestamp);
        // Connection established message doesn't need a specific handler
        // It's just confirmation that the WebSocket is working
        break;

      default:
        console.log('Unknown WebSocket message type:', (message as any).type);
    }
  }

  /**
   * Schedule reconnection with exponential backoff
   */
  private scheduleReconnect(): void {
    this.reconnectAttempts++;
    const delay = Math.min(this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1), 30000);

    console.log(`🔄 Scheduling WebSocket reconnection attempt ${this.reconnectAttempts}/${this.maxReconnectAttempts} in ${delay}ms`);

    setTimeout(() => {
      if (this.shouldReconnect) {
        this.connect(this.handlers).catch((error) => {
          console.error('WebSocket reconnection failed:', error);
        });
      }
    }, delay);
  }

  /**
   * Send a message to the WebSocket server
   */
  send(message: any): boolean {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      try {
        this.ws.send(JSON.stringify(message));
        return true;
      } catch (error) {
        console.error('Failed to send WebSocket message:', error);
        return false;
      }
    } else {
      console.warn('WebSocket is not connected');
      return false;
    }
  }

  /**
   * Disconnect from WebSocket
   */
  disconnect(): void {
    this.shouldReconnect = false;
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
  }

  /**
   * Get current connection status
   */
  get isConnected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }

  /**
   * Get current connection state
   */
  get connectionState(): string {
    if (!this.ws) return 'disconnected';

    switch (this.ws.readyState) {
      case WebSocket.CONNECTING:
        return 'connecting';
      case WebSocket.OPEN:
        return 'connected';
      case WebSocket.CLOSING:
        return 'closing';
      case WebSocket.CLOSED:
        return 'closed';
      default:
        return 'unknown';
    }
  }
}

/**
 * Create a WebSocket service instance
 */
export function createOrderbookWebSocket(url?: string): OrderbookWebSocketService {
  const wsUrl = url || process.env.NEXT_PUBLIC_ORDERBOOK_WS_URL || 'ws://localhost:8080/ws';
  return new OrderbookWebSocketService(wsUrl);
}