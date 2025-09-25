'use client';

import React, { useState, useEffect, useCallback } from 'react';
import { useWallet } from '@/components/near-wallet';
import { TinderCardStack } from '@/components/tinder-card-stack';
import { Market } from '@/lib/near';
import { marketService } from '@/services/market';
import { generateIntentId } from '@/lib/utils';
import { 
  Star, 
  Zap, 
  Target, 
  Users, 
  TrendingUp,
  Award,
  Crown,
  Medal
} from 'lucide-react';

interface UserStats {
  level: string;
  currentStreak: number;
  longestStreak: number;
  totalPredictions: number;
  correctPredictions: number;
  points: number;
  rank: number;
}

export function PredictionSwipeInterface() {
  const { nearService, isSignedIn } = useWallet();
  
  // Real markets data from NEAR contracts
  const [markets, setMarkets] = useState<Market[]>([]);
  const [marketsLoading, setMarketsLoading] = useState(true);

  const [userStats, setUserStats] = useState<UserStats>({
    level: 'Oracle',
    currentStreak: 7,
    longestStreak: 12,
    totalPredictions: 45,
    correctPredictions: 31,
    points: 2840,
    rank: 127
  });

  const [achievements, setAchievements] = useState([
    { id: 'first_prediction', title: 'First Steps', description: 'Made your first prediction', unlocked: true },
    { id: 'streak_5', title: 'On Fire', description: '5 predictions in a row', unlocked: true },
    { id: 'streak_10', title: 'Oracle Rising', description: '10 predictions in a row', unlocked: false },
    { id: 'perfect_week', title: 'Perfect Week', description: 'All predictions correct for 7 days', unlocked: false }
  ]);

  const [showStats, setShowStats] = useState(false);
  const [prediction, setPrediction] = useState<{direction: 'yes' | 'no' | 'skip', market: Market} | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  // Load real markets from NEAR contracts
  useEffect(() => {
    loadMarkets();
  }, []);

  const loadMarkets = async () => {
    console.log('[SwipeInterface] Loading real markets from NEAR contracts...');
    setMarketsLoading(true);
    try {
      const fetchedMarkets = await marketService.getMarkets(undefined, true); // Only active markets
      console.log('[SwipeInterface] Loaded', fetchedMarkets.length, 'real markets');
      setMarkets(fetchedMarkets);
    } catch (error) {
      console.error('[SwipeInterface] Error loading markets:', error);
      setMarkets([]);
    } finally {
      setMarketsLoading(false);
    }
  };


  const handleSwipe = useCallback(async (direction: 'left' | 'right' | 'up', market: Market) => {
    // Convert Tinder directions to our prediction system
    const predictionDirection = direction === 'right' ? 'yes' : direction === 'left' ? 'no' : 'skip';
    
    setPrediction({ direction: predictionDirection, market });

    // Update stats immediately for instant feedback
    if (predictionDirection !== 'skip') {
      setUserStats(prev => ({
        ...prev,
        currentStreak: prev.currentStreak + 1,
        totalPredictions: prev.totalPredictions + 1,
        points: prev.points + 10
      }));

      // Check for achievements
      if (userStats.currentStreak + 1 === 10) {
        setAchievements(prev => 
          prev.map(a => a.id === 'streak_10' ? { ...a, unlocked: true } : a)
        );
        showAchievementNotification('Oracle Rising', 'You hit a 10 prediction streak!');
      }
    }

    // Submit prediction to smart contract if user is signed in
    if (isSignedIn && predictionDirection !== 'skip') {
      setIsSubmitting(true);
      try {
        const intent = {
          intent_id: generateIntentId(),
          user: nearService.getAccountId() || '',
          market_id: market.market_id,
          intent_type: 'BuyShares' as const,
          outcome: predictionDirection === 'yes' ? 1 : 0,
          amount: nearService.parseUsdcAmount('1'), // Micro-stake: $1
          deadline: String((Date.now() + 3600000) * 1000000),
          order_type: 'Market' as const
        };

        await nearService.submitIntent(intent, 'solver.ashpk20.testnet');
      } catch (error) {
        console.error('Error submitting prediction:', error);
      } finally {
        setIsSubmitting(false);
      }
    }

    // Clear prediction after processing
    setTimeout(() => {
      setPrediction(null);
    }, 500);
  }, [markets, isSignedIn, nearService, userStats.currentStreak]);

  const showAchievementNotification = (title: string, description: string) => {
    // In a real app, this would show a toast notification
    console.log(`🏆 Achievement Unlocked: ${title} - ${description}`);
  };

  const getLevelInfo = (level: string) => {
    const levels = {
      'Novice': { icon: Star, color: 'text-gray-400', next: 'Predictor' },
      'Predictor': { icon: Target, color: 'text-blue-400', next: 'Analyst' },
      'Analyst': { icon: TrendingUp, color: 'text-green-400', next: 'Expert' },
      'Expert': { icon: Award, color: 'text-purple-400', next: 'Oracle' },
      'Oracle': { icon: Crown, color: 'text-yellow-400', next: 'Legend' },
      'Legend': { icon: Medal, color: 'text-red-400', next: 'Legend' }
    };
    return levels[level as keyof typeof levels] || levels.Novice;
  };

  const levelInfo = getLevelInfo(userStats.level);
  const LevelIcon = levelInfo.icon;

  return (
    <div className="relative">
      {/* Loading State */}
      {marketsLoading ? (
        <div className="flex items-center justify-center h-screen">
          <div className="text-center">
            <div className="w-12 h-12 border-4 border-red-500 border-t-transparent rounded-full animate-spin mx-auto mb-4"></div>
            <div className="text-white text-lg font-semibold">Loading Markets...</div>
            <div className="text-gray-400 text-sm">Fetching real markets from NEAR contracts</div>
          </div>
        </div>
      ) : markets.length === 0 ? (
        <div className="flex items-center justify-center h-screen">
          <div className="text-center max-w-sm mx-auto p-6">
            <div className="text-6xl mb-4">📊</div>
            <div className="text-white text-xl font-bold mb-2">No Markets Available</div>
            <div className="text-gray-400 text-sm mb-6">
              No active markets found. Markets are loaded from deployed NEAR contracts.
            </div>
            <button
              onClick={loadMarkets}
              className="swipe-yes px-6 py-3"
            >
              Refresh Markets
            </button>
          </div>
        </div>
      ) : (
        /* Tinder-Style Card Stack */
        <TinderCardStack
          markets={markets}
          onSwipe={handleSwipe}
          currentStreak={userStats.currentStreak}
          userLevel={userStats.level}
        />
      )}

      {/* Stats Button removed per mobile UI request */}

      {/* Loading Indicator */}
      {isSubmitting && (
        <div className="absolute top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 z-20">
          <div className="bg-black/80 rounded-full p-4">
            <div className="w-8 h-8 border-2 border-red-500 border-t-transparent rounded-full animate-spin" />
          </div>
        </div>
      )}

      {/* Stats Modal */}
      {showStats && (
        <div className="fixed inset-0 z-50 bg-black/90 flex items-center justify-center p-4">
          <div className="futurecast-card max-w-md w-full p-6">
            <div className="flex items-center justify-between mb-6">
              <h2 className="text-2xl font-bold text-white">Your Stats</h2>
              <button 
                className="action-button"
                onClick={() => setShowStats(false)}
              >
                ✕
              </button>
            </div>

            {/* Level Progress */}
            <div className="mb-6 p-4 bg-gradient-to-r from-purple-600/20 to-blue-600/20 rounded-2xl">
              <div className="flex items-center gap-3 mb-3">
                <LevelIcon className={`w-8 h-8 ${levelInfo.color}`} />
                <div>
                  <div className="text-white font-bold">{userStats.level}</div>
                  <div className="text-gray-400 text-sm">Next: {levelInfo.next}</div>
                </div>
              </div>
              <div className="w-full bg-gray-700 rounded-full h-2">
                <div 
                  className="bg-gradient-to-r from-purple-500 to-blue-500 h-2 rounded-full"
                  style={{ width: '73%' }}
                />
              </div>
            </div>

            {/* Stats Grid */}
            <div className="grid grid-cols-2 gap-4 mb-6">
              <div className="text-center p-3 bg-gray-800/50 rounded-xl">
                <div className="text-2xl font-bold text-green-400">{userStats.currentStreak}</div>
                <div className="text-gray-400 text-sm">Current Streak</div>
              </div>
              <div className="text-center p-3 bg-gray-800/50 rounded-xl">
                <div className="text-2xl font-bold text-blue-400">{userStats.longestStreak}</div>
                <div className="text-gray-400 text-sm">Best Streak</div>
              </div>
              <div className="text-center p-3 bg-gray-800/50 rounded-xl">
                <div className="text-2xl font-bold text-purple-400">{Math.round((userStats.correctPredictions / userStats.totalPredictions) * 100)}%</div>
                <div className="text-gray-400 text-sm">Accuracy</div>
              </div>
              <div className="text-center p-3 bg-gray-800/50 rounded-xl">
                <div className="text-2xl font-bold text-orange-400">{userStats.totalPredictions}</div>
                <div className="text-gray-400 text-sm">Total Made</div>
              </div>
            </div>

            {/* Recent Achievements */}
            <div>
              <h3 className="text-white font-semibold mb-3">Recent Achievements</h3>
              <div className="space-y-2">
                {achievements.filter(a => a.unlocked).slice(-3).map(achievement => (
                  <div key={achievement.id} className="flex items-center gap-3 p-3 bg-yellow-600/20 rounded-lg">
                    <div className="w-8 h-8 bg-yellow-500 rounded-full flex items-center justify-center">
                      🏆
                    </div>
                    <div>
                      <div className="text-white font-medium">{achievement.title}</div>
                      <div className="text-gray-400 text-xs">{achievement.description}</div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Tutorial Overlay for First Time Users */}
      {userStats.totalPredictions === 0 && !marketsLoading && markets.length > 0 && (
        <div className="absolute inset-0 z-30 bg-black/80 flex items-center justify-center p-4">
          <div className="futurecast-card max-w-sm text-center p-6">
            <div className="text-4xl mb-4">👋</div>
            <h2 className="text-2xl font-bold text-white mb-4">Welcome to FutureCast!</h2>
            <p className="text-gray-400 mb-6">
              Swipe through real prediction markets from NEAR contracts. Make your predictions and build your streak!
            </p>
            <div className="space-y-3">
              <div className="flex items-center gap-3 text-green-400">
                <div className="w-8 h-8 bg-green-500/20 rounded-full flex items-center justify-center">→</div>
                <span>Swipe right for YES</span>
              </div>
              <div className="flex items-center gap-3 text-red-400">
                <div className="w-8 h-8 bg-red-500/20 rounded-full flex items-center justify-center">←</div>
                <span>Swipe left for NO</span>
              </div>
              <div className="flex items-center gap-3 text-gray-400">
                <div className="w-8 h-8 bg-gray-500/20 rounded-full flex items-center justify-center">↑</div>
                <span>Swipe up to SKIP</span>
              </div>
            </div>
            <button
              className="swipe-yes w-full mt-6"
              onClick={() => setUserStats(prev => ({ ...prev, totalPredictions: 1 }))}
            >
              Let's Go! 🚀
            </button>
          </div>
        </div>
      )}
    </div>
  );
}