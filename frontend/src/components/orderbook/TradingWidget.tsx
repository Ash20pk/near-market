/**
 * Trading Widget Component
 * Compact trading interface for buy/sell orders
 */

'use client';

import React, { useState } from 'react';
import { useOrderSubmission } from '../../hooks/useOrderbook';
import { OrderSubmission } from '../../services/orderbook';

interface TradingWidgetProps {
  marketId: string;
  outcome: number;
  userAccount?: string;
  className?: string;
  onOrderSubmitted?: (orderId: string) => void;
}

export function TradingWidget({
  marketId,
  outcome,
  userAccount,
  className = '',
  onOrderSubmitted,
}: TradingWidgetProps) {
  const [side, setSide] = useState<'Buy' | 'Sell'>('Buy');
  const [price, setPrice] = useState('50.0');
  const [size, setSize] = useState('100');
  const [orderType, setOrderType] = useState<'Market' | 'Limit'>('Limit');

  const { submitOrder, submitting, lastResult, clearResult } = useOrderSubmission();

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!userAccount) {
      alert('Please connect your wallet first');
      return;
    }

    const order: OrderSubmission = {
      market_id: marketId,
      condition_id: `condition_${Date.now()}`, // Generate condition ID
      user_account: userAccount,
      outcome,
      side,
      order_type: orderType,
      price: parseFloat(price),
      size: parseFloat(size),
      expires_at: null,
      solver_account: userAccount, // Use same account as solver for now
    };

    const result = await submitOrder(order);

    if (result.success && result.orderId) {
      onOrderSubmitted?.(result.orderId);
      // Reset form on success
      setPrice('50.0');
      setSize('100');
    }
  };

  const isValidOrder = userAccount &&
    parseFloat(price) > 0 && parseFloat(price) <= 100 &&
    parseFloat(size) > 0;

  return (
    <div className={`bg-gray-900/50 rounded-lg border border-gray-700 p-4 ${className}`}>
      <form onSubmit={handleSubmit} className="space-y-4">
        {/* Header */}
        <div className="flex items-center justify-between">
          <h3 className="text-sm font-medium text-white">Quick Trade</h3>
          <span className="text-xs text-gray-400">Outcome {outcome}</span>
        </div>

        {/* Buy/Sell Toggle */}
        <div className="flex rounded-lg bg-gray-800 p-1">
          <button
            type="button"
            onClick={() => setSide('Buy')}
            className={`flex-1 py-2 px-3 rounded-md text-sm font-medium transition-colors ${
              side === 'Buy'
                ? 'bg-green-600 text-white'
                : 'text-gray-400 hover:text-white'
            }`}
          >
            Buy YES
          </button>
          <button
            type="button"
            onClick={() => setSide('Sell')}
            className={`flex-1 py-2 px-3 rounded-md text-sm font-medium transition-colors ${
              side === 'Sell'
                ? 'bg-red-600 text-white'
                : 'text-gray-400 hover:text-white'
            }`}
          >
            Sell YES
          </button>
        </div>

        {/* Order Type */}
        <div className="flex rounded-lg bg-gray-800 p-1">
          <button
            type="button"
            onClick={() => setOrderType('Limit')}
            className={`flex-1 py-1.5 px-2 rounded text-xs font-medium transition-colors ${
              orderType === 'Limit'
                ? 'bg-blue-600 text-white'
                : 'text-gray-400 hover:text-white'
            }`}
          >
            Limit
          </button>
          <button
            type="button"
            onClick={() => setOrderType('Market')}
            className={`flex-1 py-1.5 px-2 rounded text-xs font-medium transition-colors ${
              orderType === 'Market'
                ? 'bg-blue-600 text-white'
                : 'text-gray-400 hover:text-white'
            }`}
          >
            Market
          </button>
        </div>

        {/* Price Input */}
        {orderType === 'Limit' && (
          <div>
            <label className="block text-xs text-gray-400 mb-1">
              Price (¢)
            </label>
            <input
              type="number"
              value={price}
              onChange={(e) => setPrice(e.target.value)}
              min="0.1"
              max="99.9"
              step="0.1"
              className="w-full px-3 py-2 bg-gray-800 border border-gray-600 rounded-md text-white text-sm focus:outline-none focus:border-blue-500"
              placeholder="50.0"
            />
          </div>
        )}

        {/* Size Input */}
        <div>
          <label className="block text-xs text-gray-400 mb-1">
            Size (tokens)
          </label>
          <input
            type="number"
            value={size}
            onChange={(e) => setSize(e.target.value)}
            min="1"
            step="1"
            className="w-full px-3 py-2 bg-gray-800 border border-gray-600 rounded-md text-white text-sm focus:outline-none focus:border-blue-500"
            placeholder="100"
          />
        </div>

        {/* Submit Button */}
        <button
          type="submit"
          disabled={!isValidOrder || submitting}
          className={`w-full py-2.5 rounded-md font-medium text-sm transition-colors ${
            isValidOrder && !submitting
              ? side === 'Buy'
                ? 'bg-green-600 hover:bg-green-700 text-white'
                : 'bg-red-600 hover:bg-red-700 text-white'
              : 'bg-gray-700 text-gray-400 cursor-not-allowed'
          }`}
        >
          {submitting ? (
            <div className="flex items-center justify-center gap-2">
              <div className="w-4 h-4 border-2 border-gray-400 border-t-white rounded-full animate-spin" />
              Submitting...
            </div>
          ) : !userAccount ? (
            'Connect Wallet'
          ) : (
            `${side} ${orderType} Order`
          )}
        </button>

        {/* Result Message */}
        {lastResult && (
          <div className={`text-xs p-2 rounded ${
            lastResult.success
              ? 'bg-green-600/20 text-green-400'
              : 'bg-red-600/20 text-red-400'
          }`}>
            {lastResult.success ? (
              <div>
                ✅ Order submitted successfully!
                {lastResult.orderId && (
                  <div className="mt-1 font-mono text-xs text-gray-400">
                    ID: {lastResult.orderId.slice(0, 8)}...
                  </div>
                )}
              </div>
            ) : (
              <div>
                ❌ {lastResult.error || 'Order submission failed'}
              </div>
            )}
            <button
              onClick={clearResult}
              className="mt-1 text-xs underline hover:no-underline"
            >
              Clear
            </button>
          </div>
        )}

        {/* Quick Actions */}
        <div className="flex gap-2 text-xs">
          <button
            type="button"
            onClick={() => setSize('10')}
            className="px-2 py-1 bg-gray-700 hover:bg-gray-600 rounded text-gray-300"
          >
            10
          </button>
          <button
            type="button"
            onClick={() => setSize('100')}
            className="px-2 py-1 bg-gray-700 hover:bg-gray-600 rounded text-gray-300"
          >
            100
          </button>
          <button
            type="button"
            onClick={() => setSize('1000')}
            className="px-2 py-1 bg-gray-700 hover:bg-gray-600 rounded text-gray-300"
          >
            1K
          </button>
        </div>
      </form>
    </div>
  );
}

/**
 * Simple Buy/Sell buttons for minimal trading
 */
interface QuickTradeButtonsProps {
  marketId: string;
  outcome: number;
  userAccount?: string;
  className?: string;
  defaultSize?: number;
  onOrderSubmitted?: (orderId: string) => void;
}

export function QuickTradeButtons({
  marketId,
  outcome,
  userAccount,
  className = '',
  defaultSize = 100,
  onOrderSubmitted,
}: QuickTradeButtonsProps) {
  const { submitOrder, submitting } = useOrderSubmission();

  const handleQuickTrade = async (side: 'Buy' | 'Sell') => {
    if (!userAccount) {
      alert('Please connect your wallet first');
      return;
    }

    const order: OrderSubmission = {
      market_id: marketId,
      condition_id: `condition_${Date.now()}`,
      user_account: userAccount,
      outcome,
      side,
      order_type: 'Market',
      price: side === 'Buy' ? 95 : 5, // Market orders use extreme prices
      size: defaultSize,
      expires_at: null,
      solver_account: userAccount,
    };

    const result = await submitOrder(order);

    if (result.success && result.orderId) {
      onOrderSubmitted?.(result.orderId);
    }
  };

  return (
    <div className={`flex gap-2 ${className}`}>
      <button
        onClick={() => handleQuickTrade('Buy')}
        disabled={!userAccount || submitting}
        className="flex-1 py-2 px-4 bg-green-600 hover:bg-green-700 disabled:bg-gray-700 disabled:text-gray-400 text-white font-medium rounded-md transition-colors text-sm"
      >
        {submitting ? '...' : 'Buy YES'}
      </button>

      <button
        onClick={() => handleQuickTrade('Sell')}
        disabled={!userAccount || submitting}
        className="flex-1 py-2 px-4 bg-red-600 hover:bg-red-700 disabled:bg-gray-700 disabled:text-gray-400 text-white font-medium rounded-md transition-colors text-sm"
      >
        {submitting ? '...' : 'Sell YES'}
      </button>
    </div>
  );
}