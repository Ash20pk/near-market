import { connect, Contract, keyStores, WalletConnection, Near } from 'near-api-js';
import { AccountView } from 'near-api-js/lib/providers/provider';

const nearConfig = {
  testnet: {
    networkId: 'testnet',
    nodeUrl: 'https://rpc.testnet.near.org',
    walletUrl: 'https://wallet.testnet.near.org',
    helperUrl: 'https://helper.testnet.near.org',
    explorerUrl: 'https://explorer.testnet.near.org',
    keyStore: typeof window !== 'undefined' ? new keyStores.BrowserLocalStorageKeyStore() : new keyStores.InMemoryKeyStore(),
  },
  mainnet: {
    networkId: 'mainnet',
    nodeUrl: 'https://rpc.mainnet.near.org',
    walletUrl: 'https://wallet.near.org',
    helperUrl: 'https://helper.mainnet.near.org',
    explorerUrl: 'https://explorer.mainnet.near.org',
    keyStore: typeof window !== 'undefined' ? new keyStores.BrowserLocalStorageKeyStore() : new keyStores.InMemoryKeyStore(),
  },
};

export type Market = {
  market_id: string;
  condition_id: string;
  title: string;
  description: string;
  creator: string;
  end_time: string;
  resolution_time: string;
  category: string;
  is_active: boolean;
  resolver: string;
  total_volume?: string;
  created_at?: string;
};

export type PredictionIntent = {
  intent_id: string;
  user: string;
  market_id: string;
  intent_type: 'BuyShares' | 'SellShares' | 'MintComplete' | 'RedeemWinning';
  outcome: number;
  amount: string;
  max_price?: number;
  min_price?: number;
  deadline: string;
  order_type: 'Market' | 'Limit';
  cross_chain?: {
    source_chain_id: number;
    source_user: string;
    source_token: string;
    bridge_min_amount: string;
    return_to_source: boolean;
  };
};

export type ExecutionResult = {
  intent_id: string;
  success: boolean;
  output_amount?: string;
  fee_amount: string;
  execution_details: string;
};

export class NearService {
  private near: Near | null = null;
  private wallet: WalletConnection | null = null;
  private verifierContract: Contract | null = null;
  private ctfContract: Contract | null = null;
  private solverContract: Contract | null = null;
  private resolverContract: Contract | null = null;

  constructor(
    private network: 'testnet' | 'mainnet' = 'testnet',
    private contracts = {
      verifier: process.env.NEXT_PUBLIC_VERIFIER_CONTRACT || 'verifier.ashpk20.testnet',
      ctf: process.env.NEXT_PUBLIC_CTF_CONTRACT || 'ctf.ashpk20.testnet',
      solver: process.env.NEXT_PUBLIC_SOLVER_CONTRACT || 'solver.ashpk20.testnet',
      resolver: process.env.NEXT_PUBLIC_RESOLVER_CONTRACT || 'resolver.ashpk20.testnet'
    }
  ) {
    console.log('[NearService] Initialized with contracts:', this.contracts);
  }

  async initialize() {
    if (typeof window === 'undefined') {
      console.log('[NearService] Server-side environment detected - initializing for server use');
      await this.initializeServerSide();
      return;
    }

    console.log('[NearService] 🚀 Initializing NEAR connection...');
    console.log('[NearService] Network:', this.network);
    console.log('[NearService] Contract addresses:', this.contracts);

    const config = nearConfig[this.network];
    console.log('[NearService] NEAR config:', {
      networkId: config.networkId,
      nodeUrl: config.nodeUrl,
      walletUrl: config.walletUrl
    });

    this.near = await connect(config);
    this.wallet = new WalletConnection(this.near, 'prediction-market');

    console.log('[NearService] ✅ NEAR connection established');
    console.log('[NearService] User signed in:', this.wallet.isSignedIn());

    if (this.wallet.isSignedIn()) {
      console.log('[NearService] 📝 User is signed in, full access available');
      console.log('[NearService] Account ID:', this.wallet.getAccountId());
    } else {
      console.log('[NearService] 👁️ User not signed in - view methods still available');
    }

    console.log('[NearService] ✅ NEAR service ready for view function calls');
  }

  /**
   * Initialize NEAR connection for server-side use
   */
  async initializeServerSide() {
    console.log('[NearService] 🖥️ Initializing for server-side use...');

    const config = {
      networkId: this.network,
      nodeUrl: nearConfig[this.network].nodeUrl,
      keyStore: new keyStores.InMemoryKeyStore(),
    };

    console.log('[NearService] Server-side config:', { networkId: config.networkId, nodeUrl: config.nodeUrl });

    try {
      this.near = await connect(config);
      console.log('[NearService] ✅ Server-side NEAR connection established');

      // Initialize server-side view-only contracts
      console.log('[NearService] 🖥️ Initializing server-side view-only contracts...');
      this.initializeViewOnlyContractsServerSide();
    } catch (error) {
      console.error('[NearService] ❌ Failed to initialize server-side NEAR connection:', error);
      throw error;
    }
  }

  /**
   * Initialize view-only contracts for server-side use
   */
  private initializeViewOnlyContractsServerSide() {
    if (!this.near) {
      console.log('[NearService] ❌ Cannot initialize server-side contracts - NEAR instance missing');
      return;
    }

    try {
      // For server-side, we can't use this.near.account('') as it may not work properly
      // Instead, we'll use direct provider queries in the getMarkets method
      console.log('[NearService] Server-side contracts will use direct provider queries');
      console.log('[NearService] ✅ Server-side view-only setup complete');
    } catch (error) {
      console.error('[NearService] ❌ Failed to initialize server-side contracts:', error);
      throw error;
    }
  }


  /**
   * Initialize view-only contracts for unsigned users
   * These can call view methods without requiring wallet authentication
   */
  private initializeViewOnlyContracts() {
    if (!this.near) {
      console.log('[NearService] ❌ Cannot initialize view-only contracts - NEAR instance missing');
      return;
    }

    console.log('[NearService] 👁️ Initializing view-only contracts for public access...');

    // Create a temporary account for view methods (doesn't need to be signed in)
    const account = this.near.account('');

    // Verifier contract (view-only)
    console.log('[NearService] Initializing view-only Verifier contract at:', this.contracts.verifier);
    this.verifierContract = new Contract(
      account,
      this.contracts.verifier,
      {
        viewMethods: [
          'get_market',
          'get_markets',
          'is_intent_verified',
          'get_verified_intents',
          'get_execution_result',
          'is_intent_pending',
          'get_platform_config'
        ],
        changeMethods: [] // No change methods for unsigned users
      }
    );
    console.log('[NearService] ✅ View-only Verifier contract initialized');

    // CTF contract (view-only)
    this.ctfContract = new Contract(
      account,
      this.contracts.ctf,
      {
        viewMethods: [
          'get_condition',
          'is_condition_resolved',
          'balance_of',
          'get_position_id',
          'get_collection_id',
          'get_user_positions',
          'get_position'
        ],
        changeMethods: []
      }
    );
    console.log('[NearService] ✅ View-only CTF contract initialized');
  }

  private initializeContracts() {
    if (!this.wallet || !this.near) {
      console.log('[NearService] ❌ Cannot initialize contracts - wallet or NEAR instance missing');
      return;
    }

    console.log('[NearService] 📄 Initializing full contracts with wallet access...');

    // Verifier contract
    console.log('[NearService] Initializing Verifier contract at:', this.contracts.verifier);
    this.verifierContract = new Contract(
      this.wallet.account(),
      this.contracts.verifier,
      {
        viewMethods: [
          'get_market',
          'get_markets',
          'is_intent_verified',
          'get_verified_intents',
          'get_execution_result',
          'is_intent_pending',
          'get_platform_config'
        ],
        changeMethods: [
          'create_market',
          'verify_and_solve',
          'set_market_status'
        ]
      }
    );
    console.log('[NearService] ✅ Verifier contract initialized');

    // CTF contract  
    this.ctfContract = new Contract(
      this.wallet.account(),
      this.contracts.ctf,
      {
        viewMethods: [
          'get_condition',
          'is_condition_resolved',
          'balance_of',
          'get_position_id',
          'get_collection_id',
          'get_user_positions',
          'get_position'
        ],
        changeMethods: [
          'split_position',
          'merge_positions', 
          'redeem_positions',
          'safe_transfer_from',
          'approve',
          'set_approval_for_all'
        ]
      }
    );

    // Solver contract
    this.solverContract = new Contract(
      this.wallet.account(),
      this.contracts.solver,
      {
        viewMethods: [
          'get_processed_intents_count',
          'is_intent_processed',
          'get_order',
          'get_user_orders',
          'is_cross_chain_enabled'
        ],
        changeMethods: [
          'solve_intent',
          'cancel_order'
        ]
      }
    );

    // Resolver contract
    this.resolverContract = new Contract(
      this.wallet.account(),
      this.contracts.resolver,
      {
        viewMethods: [
          'get_resolution',
          'get_dispute',
          'is_market_finalized',
          'get_pending_resolutions'
        ],
        changeMethods: [
          'submit_resolution',
          'dispute_resolution',
          'finalize_resolution'
        ]
      }
    );
  }

  // Wallet methods
  async signIn() {
    if (!this.wallet) return;
    await this.wallet.requestSignIn({
      contractId: this.contracts.verifier,
      methodNames: []
    });

    // After successful sign-in, initialize full contracts
    if (this.wallet.isSignedIn()) {
      console.log('[NearService] 🔄 User signed in, upgrading to full contracts...');
      this.initializeContracts();
    }
  }

  signOut() {
    if (!this.wallet) return;
    this.wallet.signOut();
    window.location.reload();
  }

  isSignedIn(): boolean {
    return this.wallet?.isSignedIn() ?? false;
  }

  getAccountId(): string | null {
    return this.wallet?.getAccountId() ?? null;
  }

  async getAccountBalance(): Promise<string> {
    if (!this.wallet?.isSignedIn()) return '0';
    const account = this.wallet.account();
    const balance = await account.getAccountBalance();
    return balance.available;
  }

  // Market methods
  async getMarkets(category?: string, isActive?: boolean): Promise<Market[]> {
    console.log('[NearService] 📊 getMarkets called with:', { category, isActive });
    console.log('[NearService] Contract address:', this.contracts.verifier);
    console.log('[NearService] NEAR connection:', !!this.near);
    console.log('[NearService] Wallet signed in:', this.isSignedIn());

    if (!this.near) {
      console.log('[NearService] ❌ NEAR connection not established');
      return [];
    }

    try {
      console.log('[NearService] 📡 Using direct viewFunction call to:', this.contracts.verifier);

      // Use provider query directly - browser compatible
      console.log('[NearService] 📡 Calling get_markets via provider query');

      const args = JSON.stringify({
        category,
        is_active: isActive
      });

      // Convert args to base64
      const argsBase64 = btoa(args);

      const result = await this.near.connection.provider.query({
        request_type: 'call_function',
        account_id: this.contracts.verifier,
        method_name: 'get_markets',
        args_base64: argsBase64,
        finality: 'final'
      });

      console.log('[NearService] Raw result:', result);

      // Decode the result
      const resultString = new TextDecoder().decode(new Uint8Array(result.result));
      console.log('[NearService] Decoded result string:', resultString);

      const markets = JSON.parse(resultString);

      console.log('[NearService] ✅ Successfully fetched', markets?.length || 0, 'markets from NEAR contract');

      if (markets && markets.length > 0) {
        console.log('[NearService] 📋 Sample market:', {
          id: markets[0].market_id,
          title: markets[0].title?.slice(0, 50) + '...',
          active: markets[0].is_active,
          category: markets[0].category
        });
      }

      return markets || [];
    } catch (error) {
      console.error('[NearService] ❌ Error fetching markets from contract:', error);
      console.error('[NearService] Error details:', {
        name: error instanceof Error ? error.name : 'Unknown',
        message: error instanceof Error ? error.message : String(error)
      });
      return [];
    }
  }

  async getMarket(marketId: string): Promise<Market | null> {
    console.log('[NearService] getMarket called for:', marketId);
    console.log('[NearService] Contract address:', this.contracts.verifier);
    console.log('[NearService] NEAR connection:', !!this.near);

    if (!this.near) {
      console.log('[NearService] ❌ NEAR connection not established');
      return null;
    }

    try {
      console.log('[NearService] 📡 Using direct viewFunction call to:', this.contracts.verifier);

      // Use provider query directly - browser compatible
      console.log('[NearService] 📡 Calling get_market via provider query');

      const args = JSON.stringify({
        market_id: marketId
      });

      // Convert args to base64
      const argsBase64 = btoa(args);

      const result = await this.near.connection.provider.query({
        request_type: 'call_function',
        account_id: this.contracts.verifier,
        method_name: 'get_market',
        args_base64: argsBase64,
        finality: 'final'
      });

      console.log('[NearService] Raw result:', result);

      // Decode the result
      const resultString = new TextDecoder().decode(new Uint8Array(result.result));
      console.log('[NearService] Decoded result string:', resultString);

      const market = JSON.parse(resultString);

      console.log('[NearService] Market fetch result:', market ? 'found' : 'not found');
      return market;
    } catch (error) {
      console.error('[NearService] ❌ Error fetching market from contract:', error);
      return null;
    }
  }

  async createMarket({
    title,
    description,
    endTime,
    resolutionTime,
    category,
    resolver
  }: {
    title: string;
    description: string;
    endTime: string;
    resolutionTime: string;
    category: string;
    resolver: string;
  }): Promise<string | null> {
    if (!this.verifierContract) return null;
    try {
      const result = await (this.verifierContract as any).create_market(
        {
          title,
          description,
          end_time: endTime,
          resolution_time: resolutionTime,
          category,
          resolver
        },
        '300000000000000', // 300 TGas
        '1' // 1 NEAR deposit
      );
      return result;
    } catch (error) {
      console.error('Error creating market:', error);
      return null;
    }
  }

  // Trading methods
  async submitIntent(intent: PredictionIntent, solverAccount: string): Promise<ExecutionResult | null> {
    if (!this.verifierContract) return null;
    try {
      const result = await (this.verifierContract as any).verify_and_solve(
        { intent, solver_account: solverAccount },
        '300000000000000' // 300 TGas
      );
      return result;
    } catch (error) {
      console.error('Error submitting intent:', error);
      return null;
    }
  }

  async getUserPositions(): Promise<Array<{ position_id: string; balance: string }>> {
    if (!this.ctfContract || !this.wallet?.isSignedIn()) return [];
    try {
      const accountId = this.wallet.getAccountId();
      return await (this.ctfContract as any).get_user_positions({ user: accountId });
    } catch (error) {
      console.error('Error fetching user positions:', error);
      return [];
    }
  }

  async getPositionBalance(positionId: string): Promise<string> {
    if (!this.ctfContract || !this.wallet?.isSignedIn()) return '0';
    try {
      const accountId = this.wallet.getAccountId();
      const balance = await (this.ctfContract as any).balance_of({
        owner: accountId,
        position_id: positionId
      });
      return balance || '0';
    } catch (error) {
      console.error('Error fetching position balance:', error);
      return '0';
    }
  }

  // Resolution methods
  async getResolution(marketId: string) {
    if (!this.resolverContract) return null;
    try {
      return await (this.resolverContract as any).get_resolution({ market_id: marketId });
    } catch (error) {
      console.error('Error fetching resolution:', error);
      return null;
    }
  }

  async submitResolution(marketId: string, winningOutcome: number, resolutionData: string) {
    if (!this.resolverContract) return null;
    try {
      return await (this.resolverContract as any).submit_resolution(
        {
          market_id: marketId,
          winning_outcome: winningOutcome,
          resolution_data: resolutionData
        },
        '300000000000000' // 300 TGas
      );
    } catch (error) {
      console.error('Error submitting resolution:', error);
      return null;
    }
  }

  // Utility methods
  formatNearAmount(amount: string, decimals: number = 2): string {
    const nearAmount = parseFloat(amount) / 1e24;
    return nearAmount.toFixed(decimals);
  }

  parseNearAmount(amount: string): string {
    return (parseFloat(amount) * 1e24).toString();
  }

  formatUsdcAmount(amount: string, decimals: number = 2): string {
    const usdcAmount = parseFloat(amount) / 1e6; // USDC has 6 decimals
    return usdcAmount.toFixed(decimals);
  }

  parseUsdcAmount(amount: string): string {
    return Math.floor(parseFloat(amount) * 1e6).toString();
  }
}

// Export singleton instance
export const nearService = new NearService();