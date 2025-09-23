#!/usr/bin/env node

/**
 * Simple USDC Balance Checker
 * Standalone script to debug USDC balance parsing
 */

const { execSync } = require('child_process');

// Set QuikNode RPC endpoint for best performance
process.env.NEAR_CLI_TESTNET_RPC_SERVER_URL = 'https://billowing-ancient-meadow.near-testnet.quiknode.pro/02fe7cae1f78374077f55e172eba1f849e8570f4/';

const CONFIG = {
    network: 'testnet',
    usdc: '3e2210e1184b45b64c8a434c0a7e7b23cc04ea7eb7a6c3c32520d03d4afcb8af',
    account: 'ashpk20.testnet'
};

function log(message) {
    console.log(`[${new Date().toISOString()}] ${message}`);
}

async function nearView(contract, method, args = {}) {
    const argsStr = JSON.stringify(args);
    const cmd = `near view ${contract} ${method} '${argsStr}' --networkId ${CONFIG.network}`;
    
    log(`📞 NEAR View: ${contract}.${method}`);
    log(`   Command: ${cmd}`);
    
    try {
        const result = execSync(cmd, { encoding: 'utf8', timeout: 15000 });
        
        log(`   🔍 Raw CLI output:`);
        log(`   ${result}`);
        
        // Try to extract the result - look for the value after the View call line
        // Split by lines and find the line after "View call:"
        const lines = result.split('\n');
        let resultLine = null;
        
        for (let i = 0; i < lines.length; i++) {
            if (lines[i].includes('View call:') && i + 1 < lines.length) {
                resultLine = lines[i + 1].trim();
                break;
            }
        }
        
        log(`   🎯 Result line: "${resultLine}"`);
        
        if (resultLine) {
            // Try to parse the result line
            let match = resultLine.match(/^'([^']*)'$/); // Match 'quoted string'
            if (match) {
                const matchedValue = match[1];
                log(`   🎯 Matched quoted value: ${matchedValue}`);
                return matchedValue;
            }
            
            // Try to parse as JSON object/array
            match = resultLine.match(/^(\{.*\}|\[.*\])$/);
            if (match) {
                const matchedValue = match[1];
                log(`   🎯 Matched JSON value: ${matchedValue}`);
                try {
                    return JSON.parse(matchedValue);
                } catch (e) {
                    return matchedValue;
                }
            }
            
            // Try to parse as boolean or number
            if (resultLine === 'true' || resultLine === 'false') {
                return resultLine === 'true';
            }
            
            if (/^\d+$/.test(resultLine)) {
                return resultLine;
            }
        }
        
        // Fallback to old regex if new parsing fails
        const match = result.match(/('.*'|\{.*\}|\[.*\]|true|false|\d+)/s);
        if (match) {
            const matchedValue = match[0].replace(/'/g, '"');
            log(`   🎯 Matched value: ${matchedValue}`);
            
            try {
                const parsed = JSON.parse(matchedValue);
                log(`   ✅ JSON parsed result: ${parsed} (type: ${typeof parsed})`);
                return parsed;
            } catch (parseError) {
                log(`   ⚠️  JSON parse failed, returning raw: ${matchedValue}`);
                return matchedValue.replace(/"/g, '');
            }
        } else {
            log(`   ❌ No match found in result`);
            return null;
        }
    } catch (error) {
        log(`   ❌ Command failed: ${error.message}`);
        return null;
    }
}

async function checkUsdcBalance(account) {
    log(`💰 Checking USDC balance for ${account}...`);
    
    const balance = await nearView(CONFIG.usdc, 'ft_balance_of', {
        account_id: account
    });
    
    log(`   📊 Raw balance result: ${balance} (type: ${typeof balance})`);
    
    let balanceStr = '0';
    if (balance !== null && balance !== undefined) {
        if (typeof balance === 'string') {
            balanceStr = balance;
        } else if (typeof balance === 'number') {
            balanceStr = balance.toString();
        } else if (balance.toString) {
            balanceStr = balance.toString();
        } else if (typeof balance === 'object') {
            balanceStr = JSON.stringify(balance);
            log(`   🔍 Object balance: ${balanceStr}`);
        }
    }
    
    // Remove quotes
    balanceStr = balanceStr.replace(/['"]/g, '');
    
    const balanceNum = parseInt(balanceStr) || 0;
    const balanceUSDC = balanceNum / 1000000; // Convert from 6 decimals
    
    log(`   📈 Final results:`);
    log(`      • Raw string: "${balanceStr}"`);
    log(`      • Parsed number: ${balanceNum}`);
    log(`      • USDC amount: ${balanceUSDC} USDC`);
    
    return balanceStr;
}

async function main() {
    log('🔍 USDC Balance Debug Tool');
    log('='.repeat(50));
    log(`Network: ${CONFIG.network}`);
    log(`USDC Contract: ${CONFIG.usdc}`);
    log(`Account: ${CONFIG.account}`);
    log('');
    
    await checkUsdcBalance(CONFIG.account);
    
    log('');
    log('🎯 Debug complete!');
}

if (require.main === module) {
    main().catch(error => {
        console.error('❌ Debug failed:', error.message);
        process.exit(1);
    });
}

module.exports = { checkUsdcBalance };