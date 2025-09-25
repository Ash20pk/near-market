import { WalletConnector, WalletInfo, WalletState, WalletConnection } from './wallet-types';
import { NearWalletConnector } from './wallet-connectors/near-wallet';
import { MetaMaskWalletConnector } from './wallet-connectors/metamask-wallet';
import { WalletConnectConnector } from './wallet-connectors/walletconnect-wallet';

export class WalletManager {
  private connectors: Map<string, WalletConnector> = new Map();
  private currentConnection: WalletConnection | null = null;
  private state: WalletState = {
    isConnected: false,
    connection: null,
    isConnecting: false,
    error: null
  };

  constructor() {
    this.initializeConnectors();
  }

  private initializeConnectors() {
    // Initialize NEAR wallet connector
    const nearConnector = new NearWalletConnector('testnet');
    this.connectors.set('near', nearConnector);

    // Initialize MetaMask connector
    const metamaskConnector = new MetaMaskWalletConnector();
    this.connectors.set('metamask', metamaskConnector);

    // Initialize WalletConnect connector
    const walletConnectConnector = new WalletConnectConnector();
    this.connectors.set('walletconnect', walletConnectConnector);
  }

  getAvailableWallets(): WalletInfo[] {
    const wallets: WalletInfo[] = [
      {
        id: 'near',
        name: 'NEAR Wallet',
        type: 'near',
        icon: '🔗',
        description: 'Connect with your NEAR wallet',
        supportedChains: ['NEAR'],
        isInstalled: true
      },
      {
        id: 'metamask',
        name: 'MetaMask',
        type: 'metamask',
        icon: '🦊',
        description: 'Connect with MetaMask wallet',
        supportedChains: ['Ethereum', 'Polygon', 'BSC'],
        isInstalled: this.connectors.get('metamask')?.isAvailable() ?? false
      },
      {
        id: 'walletconnect',
        name: 'WalletConnect',
        type: 'walletconnect',
        icon: '📱',
        description: 'Connect with mobile wallet',
        supportedChains: ['Ethereum', 'Polygon', 'BSC', 'NEAR'],
        isInstalled: this.connectors.get('walletconnect')?.isAvailable() ?? false
      }
    ];

    return wallets.filter(wallet => wallet.isInstalled !== false);
  }

  async connect(walletType: string): Promise<WalletConnection> {
    const connector = this.connectors.get(walletType);
    if (!connector) {
      throw new Error(`Wallet connector for ${walletType} not found`);
    }

    this.state.isConnecting = true;
    this.state.error = null;

    try {
      const connection = await connector.connect();
      this.currentConnection = connection;
      this.state.isConnected = true;
      this.state.connection = connection;
      this.state.isConnecting = false;

      return connection;
    } catch (error) {
      this.state.isConnecting = false;
      this.state.error = error instanceof Error ? error.message : 'Connection failed';
      throw error;
    }
  }

  async disconnect(): Promise<void> {
    if (this.currentConnection && this.connectors.has(this.currentConnection.type)) {
      const connector = this.connectors.get(this.currentConnection.type);
      await connector?.disconnect();
    }

    this.currentConnection = null;
    this.state.isConnected = false;
    this.state.connection = null;
    this.state.error = null;
  }

  getState(): WalletState {
    return { ...this.state };
  }

  getCurrentConnection(): WalletConnection | null {
    return this.currentConnection;
  }

  isConnected(): boolean {
    return this.state.isConnected;
  }

  getAccount(): string | null {
    return this.currentConnection?.account || null;
  }

  async switchChain(chainId: string): Promise<void> {
    if (!this.currentConnection) {
      throw new Error('No wallet connected');
    }

    const connector = this.connectors.get(this.currentConnection.type);
    if (!connector || !connector.switchChain) {
      throw new Error('Chain switching not supported for this wallet');
    }

    await connector.switchChain(chainId);
  }

  async getBalance(): Promise<string> {
    if (!this.currentConnection) {
      throw new Error('No wallet connected');
    }

    const connector = this.connectors.get(this.currentConnection.type);
    if (!connector || !connector.getBalance) {
      throw new Error('Balance fetching not supported for this wallet');
    }

    return await connector.getBalance();
  }

  async signMessage(message: string): Promise<string> {
    if (!this.currentConnection) {
      throw new Error('No wallet connected');
    }

    const connector = this.connectors.get(this.currentConnection.type);
    if (!connector || !connector.signMessage) {
      throw new Error('Message signing not supported for this wallet');
    }

    return await connector.signMessage(message);
  }
}

// Export singleton instance
export const walletManager = new WalletManager();
