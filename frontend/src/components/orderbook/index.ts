/**
 * Orderbook Components - Easy Import Index
 * Import all orderbook components from one place
 */

// Core components
export {
  LivePriceDisplay,
  SimplePrice,
  PriceChange,
} from './LivePriceDisplay';

export {
  OrderbookWidget,
  MiniOrderbook,
} from './OrderbookWidget';

export {
  OrderbookChart,
  MiniOrderbookChart,
} from './OrderbookChart';

export {
  TradeHistory,
  TradeTicker,
} from './TradeHistory';

export {
  TradingWidget,
  QuickTradeButtons,
} from './TradingWidget';

// Integration examples
export {
  EnhancedMarketCard,
  EnhancedMarketDetail,
  EnhancedNavigation,
  OrderbookIntegratedApp,
} from './OrderbookIntegration';

// Services and hooks
export * from '../../services/orderbook';
export * from '../../services/websocket';
export * from '../../hooks/useOrderbook';
export * from '../../contexts/OrderbookContext';
export * from '../../config/orderbook';

// Type exports for TypeScript users
export type {
  OrderbookConfig,
  PriceData,
  OrderbookSnapshot,
  PriceLevel,
  Trade,
  OrderSubmission,
  CollateralBalance,
  WebSocketMessage,
  OrderbookUpdateMessage,
  TradeExecutedMessage,
  OrderUpdateMessage,
} from '../../services/orderbook';