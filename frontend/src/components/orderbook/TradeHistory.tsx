/**
 * Trade History Component
 * Shows recent trades with real-time updates
 */

'use client';

import React from 'react';
import { useOrderbookContext } from '../../contexts/OrderbookContext';

interface TradeHistoryProps {
  marketId?: string;
  className?: string;
  maxTrades?: number;
  showHeader?: boolean;
}

export function TradeHistory({
  marketId,
  className = '',
  maxTrades = 10,
  showHeader = true,
}: TradeHistoryProps) {
  const { state } = useOrderbookContext();

  // Filter trades by market if specified
  const trades = marketId
    ? state.recentTrades.filter(trade => trade.market_id === marketId)
    : state.recentTrades;

  const displayTrades = trades.slice(0, maxTrades);

  const formatTime = (timestamp: string) => {
    const date = new Date(timestamp);
    return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  };

  const formatSize = (size: number) => {
    if (size >= 1000) return `${(size / 1000).toFixed(1)}K`;
    return size.toFixed(0);
  };

  if (displayTrades.length === 0) {
    return (
      <div className={`text-center text-gray-500 p-4 ${className}`}>
        <div className="text-sm">No recent trades</div>
      </div>
    );
  }

  return (
    <div className={`bg-gray-900/50 rounded-lg border border-gray-700 ${className}`}>
      {showHeader && (
        <div className="px-3 py-2 border-b border-gray-700">
          <h3 className="text-sm font-medium text-white">Recent Trades</h3>
        </div>
      )}

      <div className="p-3">
        {/* Header */}
        <div className="flex justify-between text-xs text-gray-400 mb-2">
          <span>Time</span>
          <span>Price</span>
          <span>Size</span>
        </div>

        {/* Trades */}
        <div className="space-y-1 max-h-48 overflow-y-auto">
          {displayTrades.map((trade, index) => (
            <div
              key={`${trade.trade_id}-${index}`}
              className="flex justify-between text-xs hover:bg-gray-800/50 rounded px-1 py-0.5 transition-colors"
            >
              <span className="text-gray-400 font-mono">
                {formatTime(trade.executed_at)}
              </span>

              <span className={`font-medium ${
                trade.taker_side === 'Buy' ? 'text-green-400' : 'text-red-400'
              }`}>
                {trade.price.toFixed(1)}¢
              </span>

              <span className="text-gray-300">
                {formatSize(trade.size)}
              </span>
            </div>
          ))}
        </div>

        {/* Live indicator */}
        {state.isConnected && (
          <div className="flex items-center justify-center gap-1 mt-2 pt-2 border-t border-gray-700">
            <div className="w-1.5 h-1.5 bg-green-400 rounded-full animate-pulse" />
            <span className="text-xs text-gray-400">Live updates</span>
          </div>
        )}
      </div>
    </div>
  );
}

/**
 * Compact trade ticker for minimal display
 */
interface TradeTickerProps {
  marketId?: string;
  className?: string;
}

export function TradeTicker({
  marketId,
  className = '',
}: TradeTickerProps) {
  const { state } = useOrderbookContext();

  // Get the most recent trade for the market
  const recentTrade = marketId
    ? state.recentTrades.find(trade => trade.market_id === marketId)
    : state.recentTrades[0];

  if (!recentTrade) {
    return (
      <div className={`text-xs text-gray-500 ${className}`}>
        No recent trades
      </div>
    );
  }

  const formatTime = (timestamp: string) => {
    const now = Date.now();
    const tradeTime = new Date(timestamp).getTime();
    const diffSeconds = Math.floor((now - tradeTime) / 1000);

    if (diffSeconds < 60) return `${diffSeconds}s ago`;
    if (diffSeconds < 3600) return `${Math.floor(diffSeconds / 60)}m ago`;
    return `${Math.floor(diffSeconds / 3600)}h ago`;
  };

  return (
    <div className={`flex items-center gap-2 text-xs ${className}`}>
      <span className={`font-medium ${
        recentTrade.taker_side === 'Buy' ? 'text-green-400' : 'text-red-400'
      }`}>
        {recentTrade.price.toFixed(1)}¢
      </span>

      <span className="text-gray-400">
        {recentTrade.size.toFixed(0)}
      </span>

      <span className="text-gray-500">
        {formatTime(recentTrade.executed_at)}
      </span>

      {state.isConnected && (
        <div className="w-1.5 h-1.5 bg-green-400 rounded-full animate-pulse" />
      )}
    </div>
  );
}