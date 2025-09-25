/**
 * Orderbook Context Provider
 * Global state management for orderbook data and real-time updates
 */

'use client';

import React, { createContext, useContext, useReducer, useEffect, ReactNode } from 'react';
import {
  OrderbookSnapshot,
  PriceData,
  Trade,
  CollateralBalance,
  OrderSubmission,
} from '../services/orderbook';
import {
  OrderbookWebSocketService,
  createOrderbookWebSocket,
  OrderbookUpdateMessage,
  TradeExecutedMessage,
  OrderUpdateMessage,
} from '../services/websocket';

// State interface
interface OrderbookState {
  // Connection status
  isConnected: boolean;
  connectionError: string | null;

  // Market data by market_id and outcome
  orderbooks: Record<string, OrderbookSnapshot>;
  prices: Record<string, PriceData>;

  // Global trade feed
  recentTrades: Trade[];

  // User data
  balances: Record<string, CollateralBalance>; // key: accountId:marketId

  // Order tracking
  userOrders: Record<string, any>; // key: orderId

  // UI state
  selectedMarket: string | null;
  selectedOutcome: number | null;
}

// Action types
type OrderbookAction =
  | { type: 'SET_CONNECTED'; payload: boolean }
  | { type: 'SET_CONNECTION_ERROR'; payload: string | null }
  | { type: 'UPDATE_ORDERBOOK'; payload: OrderbookUpdateMessage }
  | { type: 'ADD_TRADE'; payload: TradeExecutedMessage }
  | { type: 'UPDATE_ORDER'; payload: OrderUpdateMessage }
  | { type: 'SET_PRICE'; payload: { key: string; price: PriceData } }
  | { type: 'SET_BALANCE'; payload: { key: string; balance: CollateralBalance } }
  | { type: 'SET_SELECTED_MARKET'; payload: { marketId: string | null; outcome: number | null } }
  | { type: 'CLEAR_TRADES' };

// Initial state
const initialState: OrderbookState = {
  isConnected: false,
  connectionError: null,
  orderbooks: {},
  prices: {},
  recentTrades: [],
  balances: {},
  userOrders: {},
  selectedMarket: null,
  selectedOutcome: null,
};

// Reducer
function orderbookReducer(state: OrderbookState, action: OrderbookAction): OrderbookState {
  switch (action.type) {
    case 'SET_CONNECTED':
      return {
        ...state,
        isConnected: action.payload,
        connectionError: action.payload ? null : state.connectionError,
      };

    case 'SET_CONNECTION_ERROR':
      return {
        ...state,
        connectionError: action.payload,
        isConnected: false,
      };

    case 'UPDATE_ORDERBOOK': {
      const { market_id, outcome, snapshot } = action.payload;
      const key = `${market_id}:${outcome}`;
      return {
        ...state,
        orderbooks: {
          ...state.orderbooks,
          [key]: snapshot,
        },
      };
    }

    case 'ADD_TRADE': {
      const { trade } = action.payload;
      return {
        ...state,
        recentTrades: [trade, ...state.recentTrades.slice(0, 99)], // Keep last 100 trades
      };
    }

    case 'UPDATE_ORDER': {
      const { order_id, status, filled_size } = action.payload;
      return {
        ...state,
        userOrders: {
          ...state.userOrders,
          [order_id]: {
            ...state.userOrders[order_id],
            status,
            filled_size,
            updated_at: new Date().toISOString(),
          },
        },
      };
    }

    case 'SET_PRICE': {
      const { key, price } = action.payload;
      return {
        ...state,
        prices: {
          ...state.prices,
          [key]: price,
        },
      };
    }

    case 'SET_BALANCE': {
      const { key, balance } = action.payload;
      return {
        ...state,
        balances: {
          ...state.balances,
          [key]: balance,
        },
      };
    }

    case 'SET_SELECTED_MARKET':
      return {
        ...state,
        selectedMarket: action.payload.marketId,
        selectedOutcome: action.payload.outcome,
      };

    case 'CLEAR_TRADES':
      return {
        ...state,
        recentTrades: [],
      };

    default:
      return state;
  }
}

// Context type
interface OrderbookContextType {
  state: OrderbookState;
  dispatch: React.Dispatch<OrderbookAction>;

  // Helper functions
  getOrderbook: (marketId: string, outcome: number) => OrderbookSnapshot | null;
  getPrice: (marketId: string, outcome: number) => PriceData | null;
  getBalance: (accountId: string, marketId: string) => CollateralBalance | null;
  setSelectedMarket: (marketId: string | null, outcome: number | null) => void;

  // Current market helpers
  currentOrderbook: OrderbookSnapshot | null;
  currentPrice: PriceData | null;
}

// Create context
const OrderbookContext = createContext<OrderbookContextType | undefined>(undefined);

// Provider component
interface OrderbookProviderProps {
  children: ReactNode;
}

export function OrderbookProvider({ children }: OrderbookProviderProps) {
  const [state, dispatch] = useReducer(orderbookReducer, initialState);

  // Initialize WebSocket connection
  useEffect(() => {
    console.log('[OrderbookContext] Initializing WebSocket connection...');
    const wsService = createOrderbookWebSocket();

    const connect = async () => {
      console.log('[OrderbookContext] Attempting to connect to WebSocket...');
      try {
        await wsService.connect({
          onConnect: () => {
            console.log('[OrderbookContext] ✅ WebSocket connected successfully');
            dispatch({ type: 'SET_CONNECTED', payload: true });
          },

          onDisconnect: () => {
            console.log('[OrderbookContext] 🔌 WebSocket disconnected');
            dispatch({ type: 'SET_CONNECTED', payload: false });
          },

          onError: (error) => {
            console.error('[OrderbookContext] ❌ WebSocket error occurred:', error);
            dispatch({
              type: 'SET_CONNECTION_ERROR',
              payload: 'WebSocket connection failed',
            });
          },

          onOrderbookUpdate: (message: OrderbookUpdateMessage) => {
            console.log('[OrderbookContext] 📊 Received orderbook update for:', message.market_id, 'outcome:', message.outcome);
            console.log('[OrderbookContext] Orderbook snapshot - bids:', message.snapshot.bids.length, 'asks:', message.snapshot.asks.length);
            dispatch({ type: 'UPDATE_ORDERBOOK', payload: message });
          },

          onTradeExecuted: (message: TradeExecutedMessage) => {
            console.log('[OrderbookContext] 💰 Trade executed:', message.trade.trade_id, 'size:', message.trade.size, 'price:', message.trade.price);
            dispatch({ type: 'ADD_TRADE', payload: message });
          },

          onOrderUpdate: (message: OrderUpdateMessage) => {
            console.log('[OrderbookContext] 📝 Order update:', message.order_id, 'status:', message.status, 'filled:', message.filled_size);
            dispatch({ type: 'UPDATE_ORDER', payload: message });
          },
        });
      } catch (error) {
        console.error('[OrderbookContext] ❌ Failed to connect to WebSocket:', error);
        dispatch({
          type: 'SET_CONNECTION_ERROR',
          payload: 'Failed to connect to orderbook service',
        });
      }
    };

    connect();

    return () => {
      wsService.disconnect();
    };
  }, []);

  // Helper functions
  const getOrderbook = (marketId: string, outcome: number): OrderbookSnapshot | null => {
    const key = `${marketId}:${outcome}`;
    return state.orderbooks[key] || null;
  };

  const getPrice = (marketId: string, outcome: number): PriceData | null => {
    const key = `${marketId}:${outcome}`;
    return state.prices[key] || null;
  };

  const getBalance = (accountId: string, marketId: string): CollateralBalance | null => {
    const key = `${accountId}:${marketId}`;
    return state.balances[key] || null;
  };

  const setSelectedMarket = (marketId: string | null, outcome: number | null) => {
    dispatch({
      type: 'SET_SELECTED_MARKET',
      payload: { marketId, outcome },
    });
  };

  // Current market data
  const currentOrderbook = state.selectedMarket && state.selectedOutcome !== null
    ? getOrderbook(state.selectedMarket, state.selectedOutcome)
    : null;

  const currentPrice = state.selectedMarket && state.selectedOutcome !== null
    ? getPrice(state.selectedMarket, state.selectedOutcome)
    : null;

  const contextValue: OrderbookContextType = {
    state,
    dispatch,
    getOrderbook,
    getPrice,
    getBalance,
    setSelectedMarket,
    currentOrderbook,
    currentPrice,
  };

  return (
    <OrderbookContext.Provider value={contextValue}>
      {children}
    </OrderbookContext.Provider>
  );
}

// Hook to use the orderbook context
export function useOrderbookContext() {
  const context = useContext(OrderbookContext);
  if (context === undefined) {
    throw new Error('useOrderbookContext must be used within an OrderbookProvider');
  }
  return context;
}

// Export helper functions for external use
export const orderbookHelpers = {
  /**
   * Generate a key for storing market-specific data
   */
  getMarketKey: (marketId: string, outcome: number) => `${marketId}:${outcome}`,

  /**
   * Generate a key for storing user-specific data
   */
  getUserKey: (accountId: string, marketId: string) => `${accountId}:${marketId}`,

  /**
   * Parse a market key back to components
   */
  parseMarketKey: (key: string) => {
    const [marketId, outcomeStr] = key.split(':');
    return { marketId, outcome: parseInt(outcomeStr, 10) };
  },

  /**
   * Parse a user key back to components
   */
  parseUserKey: (key: string) => {
    const [accountId, marketId] = key.split(':', 2);
    return { accountId, marketId };
  },
};