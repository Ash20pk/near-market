import { connect, Contract, keyStores, WalletConnection, Near } from 'near-api-js';
import { WalletConnector, WalletConnection as WalletConnectionType } from './wallet-types';

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

export class NearWalletConnector implements WalletConnector {
  type = 'near' as const;
  name = 'NEAR Wallet';

  private near: Near | null = null;
  private wallet: WalletConnection | null = null;
  private network: 'testnet' | 'mainnet' = 'testnet';
  private contracts = {
    verifier: process.env.NEXT_PUBLIC_VERIFIER_CONTRACT || 'verifier.ashpk20.testnet',
    ctf: process.env.NEXT_PUBLIC_CTF_CONTRACT || 'ctf.ashpk20.testnet',
    solver: process.env.NEXT_PUBLIC_SOLVER_CONTRACT || 'solver.ashpk20.testnet',
    resolver: process.env.NEXT_PUBLIC_RESOLVER_CONTRACT || 'resolver.ashpk20.testnet'
  };

  constructor(network: 'testnet' | 'mainnet' = 'testnet') {
    this.network = network;
  }

  isAvailable(): boolean {
    return typeof window !== 'undefined';
  }

  async connect(): Promise<WalletConnectionType> {
    if (typeof window === 'undefined') {
      throw new Error('NEAR wallet connection is only available in browser');
    }

    const config = nearConfig[this.network];
    this.near = await connect(config);
    this.wallet = new WalletConnection(this.near, 'prediction-market');

    if (!this.wallet.isSignedIn()) {
      await this.wallet.requestSignIn({
        contractId: this.contracts.verifier,
        methodNames: []
      });
    }

    const accountId = this.wallet.getAccountId();
    if (!accountId) {
      throw new Error('Failed to get account ID after NEAR wallet connection');
    }

    return {
      type: 'near',
      account: accountId,
      provider: this.wallet
    };
  }

  async disconnect(): Promise<void> {
    if (this.wallet) {
      this.wallet.signOut();
    }
    this.near = null;
    this.wallet = null;
  }

  async getBalance(): Promise<string> {
    if (!this.wallet?.isSignedIn()) {
      throw new Error('Wallet not connected');
    }
    const account = this.wallet.account();
    const balance = await account.getAccountBalance();
    return balance.available;
  }

  async signMessage(message: string): Promise<string> {
    if (!this.wallet?.isSignedIn()) {
      throw new Error('Wallet not connected');
    }
    // NEAR doesn't have a standard signMessage method like Ethereum
    // This would need to be implemented based on specific requirements
    throw new Error('signMessage not implemented for NEAR wallet');
  }

  // Getters for internal use
  getNear(): Near | null {
    return this.near;
  }

  getWallet(): WalletConnection | null {
    return this.wallet;
  }

  getContracts() {
    return this.contracts;
  }
}
