/**
 * Orderbook Integration Examples
 * Shows how to integrate orderbook components into existing UI without breaking it
 */

'use client';

import React from 'react';
import { OrderbookProvider } from '../../contexts/OrderbookContext';
import { LivePriceDisplay, SimplePrice, PriceChange } from './LivePriceDisplay';
import { OrderbookWidget, MiniOrderbook } from './OrderbookWidget';
import { TradeHistory, TradeTicker } from './TradeHistory';
import { TradingWidget, QuickTradeButtons } from './TradingWidget';

// Example market data (replace with real data from your app)
const EXAMPLE_MARKET = {
  id: 'market_1758568175377138284_ashpk20.testnet',
  title: 'Bitcoin will reach $100K by EOY 2024',
  outcome: 1, // YES outcome
};

/**
 * Market Card Enhancement Example
 * Shows how to add live pricing to existing market cards
 */
export function EnhancedMarketCard({
  market = EXAMPLE_MARKET,
  children
}: {
  market?: typeof EXAMPLE_MARKET,
  children?: React.ReactNode
}) {
  return (
    <div className="futurecast-card p-4">
      {/* Original card content */}
      <div className="mb-3">
        <h3 className="text-lg font-semibold text-white mb-2">{market.title}</h3>
        {children}
      </div>

      {/* Enhanced with live data */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          {/* Live price display */}
          <LivePriceDisplay
            marketId={market.id}
            outcome={market.outcome}
            className="text-sm"
          />

          {/* Price change indicator */}
          <PriceChange
            marketId={market.id}
            outcome={market.outcome}
          />
        </div>

        {/* Mini orderbook */}
        <MiniOrderbook
          marketId={market.id}
          outcome={market.outcome}
          className="text-right"
        />
      </div>

      {/* Recent trade ticker */}
      <div className="mt-2 pt-2 border-t border-gray-700">
        <TradeTicker
          marketId={market.id}
          className="flex justify-center"
        />
      </div>
    </div>
  );
}

/**
 * Market Detail Enhancement Example
 * Shows how to add orderbook and trading to market detail pages
 */
export function EnhancedMarketDetail({
  market = EXAMPLE_MARKET,
  userAccount,
  children
}: {
  market?: typeof EXAMPLE_MARKET,
  userAccount?: string,
  children?: React.ReactNode
}) {
  return (
    <div className="space-y-6">
      {/* Original market detail content */}
      <div className="futurecast-card p-6">
        <h1 className="text-2xl font-bold text-white mb-4">{market.title}</h1>
        {children}

        {/* Enhanced price display */}
        <div className="mt-4 p-4 bg-gray-800/50 rounded-lg">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm text-gray-400">Current Price</span>
            <PriceChange marketId={market.id} outcome={market.outcome} />
          </div>
          <LivePriceDisplay
            marketId={market.id}
            outcome={market.outcome}
            className="text-lg"
            showChange={false}
          />
        </div>
      </div>

      {/* Enhanced with orderbook data */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Orderbook */}
        <OrderbookWidget
          marketId={market.id}
          outcome={market.outcome}
          maxLevels={8}
        />

        {/* Recent trades */}
        <TradeHistory
          marketId={market.id}
          maxTrades={12}
        />
      </div>

      {/* Trading interface */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Full trading widget */}
        <div className="lg:col-span-2">
          <TradingWidget
            marketId={market.id}
            outcome={market.outcome}
            userAccount={userAccount}
            onOrderSubmitted={(orderId) => {
              console.log('Order submitted:', orderId);
              // Handle order submission (show notification, etc.)
            }}
          />
        </div>

        {/* Quick trade buttons */}
        <div className="space-y-4">
          <h3 className="text-sm font-medium text-white">Quick Trade</h3>
          <QuickTradeButtons
            marketId={market.id}
            outcome={market.outcome}
            userAccount={userAccount}
            defaultSize={100}
            onOrderSubmitted={(orderId) => {
              console.log('Quick order submitted:', orderId);
            }}
          />

          {/* Simple price for reference */}
          <div className="text-center">
            <div className="text-xs text-gray-400">Market Price</div>
            <SimplePrice
              marketId={market.id}
              outcome={market.outcome}
              className="text-xl text-white"
            />
          </div>
        </div>
      </div>
    </div>
  );
}

/**
 * Navigation Enhancement Example
 * Shows how to add live data to navigation/header
 */
export function EnhancedNavigation({
  featuredMarkets = [EXAMPLE_MARKET],
  children
}: {
  featuredMarkets?: typeof EXAMPLE_MARKET[],
  children?: React.ReactNode
}) {
  return (
    <div className="bg-black/90 backdrop-blur-xl border-b border-gray-800">
      {/* Original navigation content */}
      {children}

      {/* Enhanced with live market tickers */}
      <div className="px-4 py-2 overflow-x-auto">
        <div className="flex items-center gap-6 min-w-max">
          {featuredMarkets.map((market, index) => (
            <div key={market.id} className="flex items-center gap-2 text-sm">
              <span className="text-gray-400 truncate max-w-32">
                {market.title.split(' ').slice(0, 3).join(' ')}...
              </span>
              <SimplePrice
                marketId={market.id}
                outcome={market.outcome}
                className="text-white font-medium"
              />
              <PriceChange
                marketId={market.id}
                outcome={market.outcome}
              />
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

/**
 * Complete Integration Example
 * Shows how to wrap your entire app with orderbook functionality
 */
export function OrderbookIntegratedApp({ children }: { children: React.ReactNode }) {
  return (
    <OrderbookProvider>
      <div className="min-h-screen bg-gray-900">
        {/* Your existing app structure */}
        {children}

        {/* Optional: Global connection status indicator */}
        <OrderbookConnectionStatus />
      </div>
    </OrderbookProvider>
  );
}

/**
 * Connection Status Indicator
 * Shows orderbook connection status in corner
 */
function OrderbookConnectionStatus() {
  const [showDetails, setShowDetails] = React.useState(false);

  return (
    <div className="fixed bottom-4 right-4 z-50">
      <button
        onClick={() => setShowDetails(!showDetails)}
        className="w-3 h-3 rounded-full bg-green-400 animate-pulse"
        title="Orderbook connection status"
      />

      {showDetails && (
        <div className="absolute bottom-full right-0 mb-2 p-2 bg-black/90 rounded-lg border border-gray-700 text-xs text-white whitespace-nowrap">
          🟢 Orderbook Connected
          <br />
          📡 Real-time updates active
        </div>
      )}
    </div>
  );
}

// Export all components for easy importing
export {
  LivePriceDisplay,
  SimplePrice,
  PriceChange,
  OrderbookWidget,
  MiniOrderbook,
  TradeHistory,
  TradeTicker,
  TradingWidget,
  QuickTradeButtons,
};