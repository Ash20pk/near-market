'use client';

import React, { useState, useEffect, useRef } from 'react';
import { Market } from '@/lib/near';
import { formatRelativeTime, formatCurrency } from '@/lib/utils';
import { 
  Heart, 
  X, 
  Star, 
  RotateCcw,
  Clock, 
  Users, 
  TrendingUp,
  Target,
  Flame
} from 'lucide-react';
import { useRouter } from 'next/navigation';

interface TinderCardStackProps {
  markets: Market[];
  onSwipe: (direction: 'left' | 'right' | 'up', market: Market) => void;
  currentStreak: number;
  userLevel: string;
}

interface SwipeData {
  x: number;
  y: number;
  rotation: number;
  opacity: number;
}

export function TinderCardStack({ 
  markets, 
  onSwipe, 
  currentStreak,
  userLevel 
}: TinderCardStackProps) {
  const router = useRouter();
  const [currentIndex, setCurrentIndex] = useState(0);
  const [swipeData, setSwipeData] = useState<SwipeData>({ x: 0, y: 0, rotation: 0, opacity: 1 });
  const [isAnimating, setIsAnimating] = useState(false);
  const [showIndicator, setShowIndicator] = useState<'yes' | 'no' | 'skip' | null>(null);
  const [showParticles, setShowParticles] = useState<'like' | 'nope' | null>(null);
  
  const cardRef = useRef<HTMLDivElement>(null);
  const isDragging = useRef(false);
  const startPos = useRef({ x: 0, y: 0 });
  const startTimeRef = useRef<number>(0);
  const swipeHistoryRef = useRef<Market[]>([]);

  // Get visible cards (current + next 3)
  const visibleCards = markets.slice(currentIndex, currentIndex + 4);

  const handleMouseDown = (e: React.MouseEvent) => {
    if (isAnimating) return;
    isDragging.current = true;
    startPos.current = { x: e.clientX, y: e.clientY };
    startTimeRef.current = Date.now();
    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
  };

  const handleTouchStart = (e: React.TouchEvent) => {
    if (isAnimating) return;
    isDragging.current = true;
    const touch = e.touches[0];
    startPos.current = { x: touch.clientX, y: touch.clientY };
    startTimeRef.current = Date.now();
    document.addEventListener('touchmove', handleTouchMove as any);
    document.addEventListener('touchend', handleTouchEnd);
  };

  const handleMouseMove = (e: MouseEvent) => {
    if (!isDragging.current) return;
    updateSwipeData(e.clientX, e.clientY);
  };

  const handleTouchMove = (e: TouchEvent) => {
    if (!isDragging.current) return;
    const touch = e.touches[0];
    updateSwipeData(touch.clientX, touch.clientY);
  };

  const updateSwipeData = (clientX: number, clientY: number) => {
    const deltaX = clientX - startPos.current.x;
    const deltaY = clientY - startPos.current.y;
    const rotation = deltaX * 0.1;
    const opacity = Math.max(0.5, 1 - Math.abs(deltaX) / 300);

    setSwipeData({ x: deltaX, y: deltaY, rotation, opacity });

    // Show swipe indicators
    if (Math.abs(deltaX) > 50) {
      setShowIndicator(deltaX > 0 ? 'yes' : 'no');
    } else if (deltaY < -50) {
      setShowIndicator('skip');
    } else {
      setShowIndicator(null);
    }
  };

  const handleMouseUp = () => {
    handleSwipeEnd();
  };

  const handleTouchEnd = () => {
    handleSwipeEnd();
  };

  const handleSwipeEnd = () => {
    isDragging.current = false;
    document.removeEventListener('mousemove', handleMouseMove);
    document.removeEventListener('mouseup', handleMouseUp);
    document.removeEventListener('touchmove', handleTouchMove as any);
    document.removeEventListener('touchend', handleTouchEnd);

    const threshold = 120;
    const tapMoveThreshold = 8; // px
    const tapTimeThreshold = 250; // ms
    const { x, y } = swipeData;
    const elapsed = Date.now() - startTimeRef.current;

    // Treat as tap if minimal movement and quick release
    if (Math.abs(x) < tapMoveThreshold && Math.abs(y) < tapMoveThreshold && elapsed < tapTimeThreshold) {
      const currentMarket = visibleCards[0];
      if (currentMarket) {
        console.log(`[TinderStack] Navigating to market details: ${currentMarket.market_id}`);
        console.log(`[TinderStack] Market title: ${currentMarket.title}`);
        router.push(`/market/${currentMarket.market_id}`);
        setSwipeData({ x: 0, y: 0, rotation: 0, opacity: 1 });
        setShowIndicator(null);
        return;
      }
    }

    if (Math.abs(x) > threshold) {
      // Horizontal swipe
      performSwipe(x > 0 ? 'right' : 'left');
    } else if (y < -threshold) {
      // Upward swipe
      performSwipe('up');
    } else {
      // Snap back
      setSwipeData({ x: 0, y: 0, rotation: 0, opacity: 1 });
      setShowIndicator(null);
    }
  };

  const performSwipe = (direction: 'left' | 'right' | 'up') => {
    const currentMarket = visibleCards[0];
    if (!currentMarket) {
      console.log('[TinderStack] No market to swipe - currentIndex:', currentIndex, 'markets.length:', markets.length);
      return;
    }

    console.log(`[TinderStack] Starting ${direction} swipe on market:`, currentMarket.market_id);
    setIsAnimating(true);
    setShowIndicator(null);

    // Animate card out
    const exitX = direction === 'right' ? 400 : direction === 'left' ? -400 : 0;
    const exitY = direction === 'up' ? -400 : 0;
    const exitRotation = direction === 'right' ? 30 : direction === 'left' ? -30 : 0;

    setSwipeData({ 
      x: exitX, 
      y: exitY, 
      rotation: exitRotation, 
      opacity: 0 
    });

    // Trigger haptic feedback and particles
    if (navigator.vibrate) {
      navigator.vibrate(direction === 'up' ? 50 : 100);
    }
    
    // Show particle effect
    if (direction === 'right') {
      setShowParticles('like');
    } else if (direction === 'left') {
      setShowParticles('nope');
    }

    setTimeout(() => {
      // Save history for undo
      swipeHistoryRef.current.push(currentMarket);

      // Move to next market
      const nextIndex = currentIndex + 1;
      setCurrentIndex(nextIndex);

      // Reset card state for next card
      setSwipeData({ x: 0, y: 0, rotation: 0, opacity: 1 });
      setIsAnimating(false);
      setShowParticles(null);

      // Call the parent handler
      onSwipe(direction, currentMarket);

      console.log(`[TinderStack] Swiped ${direction} on market: ${currentMarket.market_id}`);
      console.log(`[TinderStack] Moving to index ${nextIndex} of ${markets.length} total markets`);
    }, 300);
  };

  const undoLastSwipe = () => {
    if (isAnimating) return;
    if (swipeHistoryRef.current.length === 0) return;
    // Pop last swiped card and move index back by one
    swipeHistoryRef.current.pop();
    setCurrentIndex(prev => Math.max(0, prev - 1));
    setSwipeData({ x: 0, y: 0, rotation: 0, opacity: 1 });
    setShowIndicator(null);
  };

  const handleButtonAction = (action: 'reject' | 'undo' | 'like') => {
    switch (action) {
      case 'reject':
        performSwipe('left');
        break;
      case 'undo':
        undoLastSwipe();
        break;
      case 'like':
        performSwipe('right');
        break;
    }
  };

  if (currentIndex >= markets.length) {
    return (
      <div className="card-stack-container">
        <div className="text-center text-white">
          <div className="text-6xl mb-4">🎉</div>
          <h2 className="text-2xl font-bold mb-2">You've seen all predictions!</h2>
          <p className="text-gray-400">Check back later for more markets.</p>
        </div>
      </div>
    );
  }

  return (
    <div className="card-stack-container">
      {/* Header removed for a cleaner, more compact mobile card experience */}

      {/* Particle Effects */}
      {showParticles && (
        <div className="swipe-particles">
          {[...Array(12)].map((_, i) => (
            <div
              key={i}
              className={`particle ${showParticles}`}
              style={{
                '--random-x': `${(Math.random() - 0.5) * 200}px`,
                '--random-y': `${(Math.random() - 0.5) * 200}px`,
                left: `${Math.random() * 100}px`,
                top: `${Math.random() * 100}px`,
                animationDelay: `${i * 0.05}s`
              } as React.CSSProperties}
            />
          ))}
        </div>
      )}

      {/* Card Stack */}
      <div className="card-stack">
        {visibleCards.map((market, index) => (
          <div
            key={`${market.market_id}-${currentIndex + index}`}
            ref={index === 0 ? cardRef : null}
            className="prediction-card"
            style={{
              zIndex: 10 - index,
              transform: index === 0 && isDragging.current
                ? `translate(${swipeData.x}px, ${swipeData.y}px) rotate(${swipeData.rotation}deg) scale(1)`
                : index === 0 
                  ? 'translateY(0px) scale(1) rotate(0deg)'
                  : `translateY(${index * 4}px) scale(${1 - index * 0.01}) rotate(${index % 2 === 0 ? index * 0.3 : -index * 0.3}deg)`,
              opacity: index === 0 ? swipeData.opacity : Math.max(0.8, 1 - index * 0.1),
              transition: isDragging.current && index === 0 ? 'none' : 'all 0.4s cubic-bezier(0.4, 0, 0.2, 1)',
              transformOrigin: 'center center'
            }}
            onMouseDown={index === 0 ? handleMouseDown : undefined}
            onTouchStart={index === 0 ? handleTouchStart : undefined}
            onClick={index === 0 ? () => {
              // In case of mouse click without movement
              if (!isAnimating && !isDragging.current) {
                const currentMarket = visibleCards[0];
                if (currentMarket) {
                  console.log(`[TinderStack] Click navigation to market: ${currentMarket.market_id}`);
                  router.push(`/market/${currentMarket.market_id}`);
                }
              }
            } : undefined}
          >
            {/* Card Background */}
            <div 
              className="prediction-card-bg"
              style={{
                background: `linear-gradient(135deg, 
                  ${market.category === 'Crypto' ? '#ff6b35, #f7931e' :
                    market.category === 'Politics' ? '#667eea, #764ba2' :
                    market.category === 'Technology' ? '#667db6, #0082c8' :
                    '#ff9a9e, #fecfef'})`
              }}
            />
            <div className="prediction-overlay" />

            {/* Swipe Indicators */}
            {index === 0 && showIndicator && (
              <>
                <div className={`swipe-indicator ${showIndicator}`}>
                  {showIndicator === 'yes' ? 'LIKE' :
                   showIndicator === 'no' ? 'NOPE' : 'SKIP'}
                </div>
              </>
            )}

            {/* Card Content */}
            <div className="prediction-content p-5 md:p-6">
              <div className="flex-1 flex flex-col justify-center">
                <div className="mb-4">
                  <div className="bg-black/30 backdrop-blur-sm px-3 py-1 rounded-full inline-flex items-center gap-2 mb-4">
                    <Target className="w-4 h-4" />
                    <span className="text-sm font-medium">{market.category}</span>
                  </div>
                </div>

                <h1 className="text-2xl md:text-3xl font-bold text-white mb-3 md:mb-4 leading-tight">
                  {market.title}
                </h1>
                
                <p className="text-gray-200 mb-5 md:mb-6 text-base md:text-lg leading-relaxed">
                  {market.description.slice(0, 150)}...
                </p>

                <div className="grid grid-cols-2 gap-3 md:gap-4 mb-5 md:mb-6">
                  <div className="bg-black/30 backdrop-blur-sm rounded-2xl p-4">
                    <div className="flex items-center gap-2 mb-2">
                      <Clock className="w-4 h-4 text-blue-400" />
                      <span className="text-sm text-gray-300">Ends</span>
                    </div>
                    <div className="text-white font-semibold">
                      {formatRelativeTime(market.end_time)}
                    </div>
                  </div>
                  
                  <div className="bg-black/30 backdrop-blur-sm rounded-2xl p-4">
                    <div className="flex items-center gap-2 mb-2">
                      <Users className="w-4 h-4 text-green-400" />
                      <span className="text-sm text-gray-300">Volume</span>
                    </div>
                    <div className="text-white font-semibold">
                      {formatCurrency(parseFloat(market.total_volume || '50000') / 1e6)}
                    </div>
                  </div>
                </div>

                {/* Quick prediction stats */}
                <div className="bg-black/30 backdrop-blur-sm rounded-2xl p-4">
                  <div className="flex items-center justify-between mb-2">
                    <span className="text-gray-300">Community Prediction</span>
                    <TrendingUp className="w-4 h-4 text-green-400" />
                  </div>
                  <div className="w-full bg-gray-700 rounded-full h-2">
                    <div 
                      className="bg-gradient-to-r from-green-500 to-blue-500 h-2 rounded-full"
                      style={{ width: '67%' }}
                    />
                  </div>
                  <div className="flex justify-between text-xs md:text-sm text-gray-400 mt-2">
                    <span>33% NO</span>
                    <span>67% YES</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        ))}
      </div>

      {/* Tinder-Style Action Buttons: Left, Undo, Right */}
      <div className="tinder-actions">
        <button 
          className="tinder-button reject"
          aria-label="Nope"
          onClick={() => handleButtonAction('reject')}
          disabled={isAnimating}
        >
          <X className="w-6 h-6" />
        </button>
        
        <button 
          className="tinder-button skip"
          aria-label="Undo"
          onClick={() => handleButtonAction('undo')}
          disabled={isAnimating || swipeHistoryRef.current.length === 0}
        >
          <RotateCcw className="w-5 h-5" />
        </button>
        
        <button 
          className="tinder-button like"
          aria-label="Like"
          onClick={() => handleButtonAction('like')}
          disabled={isAnimating}
        >
          <Heart className="w-6 h-6" />
        </button>
      </div>
    </div>
  );
}