// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

interface IERC20 {
    function approve(address spender, uint256 amount) external returns (bool);
    function transfer(address to, uint256 amount) external returns (bool);
}

/// @title StrategyVault
/// @notice Isolated custody for a single strategy on one chain. Only the delegated owner
///         (SessionSpend7702 EOA) may move funds.
contract StrategyVault {
    address public immutable owner;

    error NotOwner();
    error CallFailed();

    modifier onlyOwner() {
        if (msg.sender != owner) revert NotOwner();
        _;
    }

    constructor(address owner_) {
        owner = owner_;
    }

    receive() external payable {}

    function approveToken(address token, address spender, uint256 amount) external onlyOwner {
        if (!IERC20(token).approve(spender, amount)) revert CallFailed();
    }

    function transferToken(address token, address to, uint256 amount) external onlyOwner {
        if (!IERC20(token).transfer(to, amount)) revert CallFailed();
    }

    function transferNative(address payable to, uint256 amount) external onlyOwner {
        (bool sent,) = to.call{value: amount}("");
        if (!sent) revert CallFailed();
    }

    function callWithValue(address target, bytes calldata data, uint256 value) external onlyOwner {
        (bool ok,) = target.call{value: value}(data);
        if (!ok) revert CallFailed();
    }

    function callExternal(address target, bytes calldata data) external onlyOwner {
        (bool ok,) = target.call(data);
        if (!ok) revert CallFailed();
    }
}
