'use client';

import React, { useState, useEffect } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Select } from '@/components/ui/select';
import { Badge } from '@/components/ui/badge';
import { useWallet } from '@/components/near-wallet';
import { Market, PredictionIntent } from '@/lib/near';
import { generateIntentId, formatCurrency, INTENT_TYPES, basisPointsToPercent } from '@/lib/utils';
import { TrendingUp, TrendingDown, Info, AlertCircle, DollarSign, Percent } from 'lucide-react';
import { useMarketPrice } from '@/hooks/useOrderbook';

interface TradingInterfaceProps {
  market: Market;
  onTradeSubmitted?: (result: any) => void;
}

type TradeMode = 'buy' | 'sell';
type OrderType = 'Market' | 'Limit';
type AdvancedTab = 'mint' | 'redeem';
type Outcome = 'YES' | 'NO';

export function TradingInterface({ market, onTradeSubmitted }: TradingInterfaceProps) {
  const { nearService, isSignedIn } = useWallet();
  const [tradeMode, setTradeMode] = useState<TradeMode>('buy');
  const [orderType, setOrderType] = useState<OrderType>('Limit'); // Default to Limit for safety
  // Removed selectedOutcome - we'll use tradeMode to determine this
  const [amount, setAmount] = useState('');
  const [limitPrice, setLimitPrice] = useState('');
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [advancedTab, setAdvancedTab] = useState<AdvancedTab>('mint');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  // Check if market has existing data/trades
  const { price: marketPriceData, loading: priceLoading } = useMarketPrice(market.market_id, 1);
  const hasMarketData = !priceLoading && marketPriceData !== null;
  const isNewMarket = !hasMarketData;

  // Auto-set order type based on market data availability
  useEffect(() => {
    if (!priceLoading) {
      if (isNewMarket) {
        // Force limit orders for new markets
        setOrderType('Limit');
        if (!limitPrice) {
          setLimitPrice('50'); // Default to 50¢ for new markets
        }
      } else if (hasMarketData && orderType === 'Limit' && !limitPrice) {
        // Allow market orders for established markets
        setOrderType('Market');
      }
    }
  }, [priceLoading, isNewMarket, hasMarketData]);

  const handleTradeModeChange = (mode: TradeMode) => {
    setTradeMode(mode);
    setError('');
  };

  const handleOrderTypeChange = (type: OrderType) => {
    // Prevent market orders for new markets without trading data
    if (type === 'Market' && isNewMarket) {
      setError('Market orders require existing trading data. Please use a limit order to establish the market.');
      return;
    }

    setOrderType(type);
    setError('');
    if (type === 'Market') {
      setLimitPrice('');
    }
  };

  const handleSubmitTrade = async () => {
    if (!isSignedIn) {
      setError('Please connect your wallet first');
      return;
    }

    if (!nearService) {
      setError('NEAR service not initialized. Please try again.');
      return;
    }

    if (!amount || parseFloat(amount) <= 0) {
      setError('Please enter a valid amount');
      return;
    }

    if (orderType === 'Limit' && (!limitPrice || parseFloat(limitPrice) <= 0)) {
      setError('Please enter a valid limit price');
      return;
    }

    // Ensure limit orders for new markets
    if (isNewMarket && orderType === 'Market') {
      setError('Market orders are not allowed for new markets without trading data. Please use a limit order.');
      return;
    }

    setLoading(true);
    setError('');

    try {
      let intentType: PredictionIntent['intent_type'];

      if (showAdvanced) {
        intentType = {
          mint: 'MintComplete',
          redeem: 'RedeemWinning'
        }[advancedTab];
      } else {
        // Both buy and sell create BuyShares intents for directional betting
        // Buy = buy YES shares, Sell = buy NO shares (bet against)
        intentType = 'BuyShares';
      }

      const usdcAmount = nearService.parseUsdcAmount(amount);
      const deadline = (Date.now() + 3600000) * 1000000;

      const intent: PredictionIntent = {
        intent_id: generateIntentId(),
        user: nearService.getAccountId() || '',
        market_id: market.market_id,
        intent_type: intentType,
        outcome: tradeMode === 'buy' ? 1 : 0, // Buy = YES (1), Sell = NO (0)
        amount: usdcAmount,
        deadline: deadline.toString(),
        order_type: orderType
      };

      if (orderType === 'Limit' && !showAdvanced) {
        // Convert cents to basis points (60¢ -> 60000)
        const priceInBasisPoints = Math.floor(parseFloat(limitPrice) * 1000);
        // Both buy and sell use max_price since they're both BuyShares intents
        // Buy = buy YES shares at max price, Sell = buy NO shares at max price
        intent.max_price = priceInBasisPoints;
      }

      const result = await nearService.submitIntent(intent, 'solver.testnet');
      
      if (result) {
        onTradeSubmitted?.(result);
        setAmount('');
        setLimitPrice('');
      } else {
        setError('Failed to submit trade. Please try again.');
      }
    } catch (err: any) {
      console.error('Trade submission error:', err);
      setError(err.message || 'An unexpected error occurred');
    } finally {
      setLoading(false);
    }
  };

  // Mock probability for display
  const probability = 0.72; // This would come from market data

  return (
    <div className="futurecast-card p-6">
      <div className="mb-6">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-xl font-bold text-white">Trade</h3>
          {!showAdvanced && (
            <button
              onClick={() => setShowAdvanced(true)}
              className="text-sm text-gray-400 hover:text-white transition-colors"
            >
              More
            </button>
          )}
          {showAdvanced && (
            <button
              onClick={() => setShowAdvanced(false)}
              className="text-sm text-gray-400 hover:text-white transition-colors"
            >
              ← Back
            </button>
          )}
        </div>

        {!showAdvanced ? (
          <>
            {/* Buy/Sell Toggle - Polymarket Style */}
            <div className="grid grid-cols-2 gap-1 bg-gray-800/50 p-1 rounded-lg mb-6">
              <button
                onClick={() => handleTradeModeChange('buy')}
                className={`py-3 px-4 text-sm font-semibold rounded-md transition-all ${
                  tradeMode === 'buy'
                    ? 'bg-green-500 text-white shadow-sm'
                    : 'text-gray-400 hover:text-white hover:bg-gray-700/50'
                }`}
              >
                Buy
              </button>
              <button
                onClick={() => handleTradeModeChange('sell')}
                className={`py-3 px-4 text-sm font-semibold rounded-md transition-all ${
                  tradeMode === 'sell'
                    ? 'bg-red-500 text-white shadow-sm'
                    : 'text-gray-400 hover:text-white hover:bg-gray-700/50'
                }`}
              >
                Sell
              </button>
            </div>

            {/* Market/Limit Toggle - Primary Tabs */}
            <div className="grid grid-cols-2 gap-1 bg-gray-800/30 p-1 rounded-lg mb-6">
              <button
                onClick={() => handleOrderTypeChange('Market')}
                disabled={isNewMarket}
                className={`py-2 px-3 text-sm font-medium rounded-md transition-all ${
                  orderType === 'Market'
                    ? 'bg-white text-black shadow-sm'
                    : isNewMarket
                      ? 'text-gray-500 cursor-not-allowed opacity-50'
                      : 'text-gray-300 hover:text-white hover:bg-gray-700/30'
                }`}
                title={isNewMarket ? 'Market orders require existing trading data' : ''}
              >
                Market
              </button>
              <button
                onClick={() => handleOrderTypeChange('Limit')}
                className={`py-2 px-3 text-sm font-medium rounded-md transition-all ${
                  orderType === 'Limit'
                    ? 'bg-white text-black shadow-sm'
                    : 'text-gray-300 hover:text-white hover:bg-gray-700/30'
                }`}
              >
                Limit
              </button>
            </div>

            {/* New Market Info */}
            {isNewMarket && (
              <div className="bg-blue-900/20 border border-blue-700/50 rounded-lg p-3 mb-4">
                <div className="flex items-center gap-2 text-blue-400 text-sm">
                  <Info className="w-4 h-4" />
                  <div>
                    <div className="font-medium">New Market</div>
                    <div className="text-blue-300/80 text-xs mt-1">
                      This market has no trading history yet. Use limit orders to establish the initial price discovery.
                    </div>
                  </div>
                </div>
              </div>
            )}
          </>
        ) : (
          <>
            {/* Advanced Operations */}
            <div className="grid grid-cols-2 gap-1 bg-gray-800/50 p-1 rounded-lg mb-6">
              <button
                onClick={() => setAdvancedTab('mint')}
                className={`py-3 px-4 text-sm font-semibold rounded-md transition-all ${
                  advancedTab === 'mint'
                    ? 'bg-blue-500 text-white shadow-sm'
                    : 'text-gray-400 hover:text-white hover:bg-gray-700/50'
                }`}
              >
                Mint
              </button>
              <button
                onClick={() => setAdvancedTab('redeem')}
                className={`py-3 px-4 text-sm font-semibold rounded-md transition-all ${
                  advancedTab === 'redeem'
                    ? 'bg-purple-500 text-white shadow-sm'
                    : 'text-gray-400 hover:text-white hover:bg-gray-700/50'
                }`}
              >
                Redeem
              </button>
            </div>
          </>
        )}


        {/* Amount Input */}
        <div className="mb-6">
          <label className="block text-sm font-medium text-gray-300 mb-3">
            Amount (USDC)
          </label>
          <div className="relative">
            <DollarSign className="absolute left-3 top-1/2 transform -translate-y-1/2 text-gray-400 w-4 h-4" />
            <input
              type="number"
              placeholder="0.00"
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
              min="0"
              step="0.01"
              className="polymarket-input pl-10 w-full"
            />
          </div>
          <div className="flex justify-between mt-2 text-xs text-gray-400">
            <span>Min: $1.00</span>
            <span>Max: $10,000</span>
          </div>
        </div>


        {/* Limit Price (for limit orders) */}
        {orderType === 'Limit' && !showAdvanced && (
          <div className="mb-6">
            <label className="block text-sm font-medium text-gray-300 mb-3">
              {tradeMode === 'buy' ? 'Max Price' : 'Min Price'} (¢)
            </label>
            <div className="relative">
              <Percent className="absolute left-3 top-1/2 transform -translate-y-1/2 text-gray-400 w-4 h-4" />
              <input
                type="number"
                placeholder="50"
                value={limitPrice}
                onChange={(e) => setLimitPrice(e.target.value)}
                min="1"
                max="99"
                step="1"
                className="polymarket-input pl-10 w-full"
              />
            </div>
            <p className="text-xs text-gray-400 mt-2">
              Price in cents (1-99¢)
            </p>
          </div>
        )}

        {/* Trade Summary */}
        <div className="bg-gray-800 p-4 rounded-lg mb-6">
          <div className="text-sm font-medium text-white mb-3">Order Summary</div>
          <div className="space-y-2 text-sm">
            <div className="flex justify-between text-gray-300">
              <span>Action:</span>
              <span className="capitalize text-white">
                {showAdvanced
                  ? INTENT_TYPES[{
                      mint: 'MintComplete',
                      redeem: 'RedeemWinning'
                    }[advancedTab] as keyof typeof INTENT_TYPES]
                  : 'Buy Shares' // Both buy and sell are BuyShares intents
                }
              </span>
            </div>

            {!showAdvanced && (
              <div className="flex justify-between text-gray-300">
                <span>Outcome:</span>
                <span className={`font-medium ${
                  tradeMode === 'buy' ? 'text-green-400' : 'text-red-400'
                }`}>
                  {tradeMode === 'buy' ? 'YES' : 'NO'}
                </span>
              </div>
            )}
            
            <div className="flex justify-between text-gray-300">
              <span>Amount:</span>
              <span className="text-white">{amount || '0'} USDC</span>
            </div>
            
            {orderType === 'Limit' && limitPrice && (
              <div className="flex justify-between text-gray-300">
                <span>Price:</span>
                <span className="text-white">{limitPrice}¢</span>
              </div>
            )}
            
            <div className="flex justify-between text-gray-300 border-t border-gray-700 pt-2">
              <span>Est. Fee:</span>
              <span className="text-white">~$0.{Math.max(1, Math.ceil((parseFloat(amount || '0') * 0.005))).toString().padStart(2, '0')}</span>
            </div>

            {amount && orderType === 'Limit' && limitPrice && (
              <div className="flex justify-between text-gray-300">
                <span>Price per share:</span>
                <span className="text-white font-medium">{limitPrice}¢</span>
              </div>
            )}

            {amount && (
              <div className="flex justify-between text-gray-300">
                <span>You'll {tradeMode === 'buy' ? 'buy' : 'sell'}:</span>
                <span className="text-white font-medium">
                  ~{orderType === 'Limit' && limitPrice
                    ? Math.floor(parseFloat(amount) / (parseFloat(limitPrice) / 100)).toLocaleString()
                    : (parseFloat(amount) / 0.50).toFixed(0)
                  } shares
                </span>
              </div>
            )}
          </div>
        </div>

        {/* Error Message */}
        {error && (
          <div className="bg-red-900/20 border border-red-700/50 rounded-lg p-4 mb-6">
            <div className="flex items-center gap-2 text-red-400 text-sm">
              <AlertCircle className="w-4 h-4" />
              {error}
            </div>
          </div>
        )}

        {/* Submit Button */}
        <button
          onClick={handleSubmitTrade}
          disabled={loading || !amount || !isSignedIn || !nearService}
          className={`w-full py-4 rounded-lg font-semibold text-white transition-all ${
            !isSignedIn || !amount || !nearService
              ? 'bg-gray-700 cursor-not-allowed'
              : loading
                ? 'bg-gray-700 cursor-not-allowed'
                : 'polymarket-button-primary'
          }`}
        >
          {!isSignedIn
            ? 'Connect Wallet'
            : !nearService
              ? 'Initializing...'
              : loading
                ? 'Submitting...'
                : showAdvanced
                  ? `${advancedTab.charAt(0).toUpperCase() + advancedTab.slice(1)} Shares`
                  : `${tradeMode.charAt(0).toUpperCase() + tradeMode.slice(1)} ${tradeMode === 'buy' ? 'YES' : 'NO'}`
          }
        </button>
      </div>
    </div>
  );
}