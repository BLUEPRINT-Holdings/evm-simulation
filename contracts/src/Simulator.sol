// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "./interfaces/IUniswapV2Pair.sol";
import "./interfaces/IUniswapV2Router02.sol";
import "./interfaces/IUniswapV3Pool.sol";
import "./interfaces/IERC20.sol";

import "./utils/SafeERC20.sol";

contract Simulator {
    using SafeERC20 for IERC20;

    function simpleTransfer(
        uint256 amount,
        address sendingToken
    ) external returns (uint256 transferedAmount) {
        // Send token from simulator (EOA) to this contract
        IERC20(sendingToken).safeTransferFrom(msg.sender, address(this), amount);
        transferedAmount = IERC20(sendingToken).balanceOf(address(this));
    }

    function v2SimulateSwap(
        uint256 amountIn,
        address targetPair,
        address inputToken,
        address outputToken
    ) external returns (uint256 targetedAmountOut, uint256 realAfterBalance) {
        // 1. Check if you can transfer the token
        // Some honeypot tokens won't allow you to transfer tokens
        IERC20(inputToken).safeTransfer(targetPair, amountIn);

        uint256 reserveIn;
        uint256 reserveOut;

        {
            (uint256 reserve0, uint256 reserve1, ) = IUniswapV2Pair(targetPair)
                .getReserves();

            if (inputToken < outputToken) {
                reserveIn = reserve0;
                reserveOut = reserve1;
            } else {
                reserveIn = reserve1;
                reserveOut = reserve0;
            }
        }

        // 2. Calculate the amount out you are supposed to get if the token isn't taxed
        // use amountIn defined as an arg to calculate the amountOut without tax
        targetedAmountOut = this.getAmountOut(amountIn, reserveIn, reserveOut);
        uint256 actualAmountIn = IERC20(inputToken).balanceOf(targetPair) -
            reserveIn;
        uint256 amountOut = this.getAmountOut(actualAmountIn, reserveIn, reserveOut);

        // If the token is taxed, you won't receive amountOut back, and the swap will revert
        uint256 outBalanceBefore = IERC20(outputToken).balanceOf(address(this));

        (uint256 amount0Out, uint256 amount1Out) = inputToken < outputToken
            ? (uint256(0), amountOut)
            : (amountOut, uint256(0));
        IUniswapV2Pair(targetPair).swap(
            amount0Out,
            amount1Out,
            address(this),
            new bytes(0)
        );

        // 3. Check the real balance of outputToken after the swap
        realAfterBalance =
            IERC20(outputToken).balanceOf(address(this)) -
            outBalanceBefore;
    }

    function getAmountOut(
        uint256 amountIn,
        uint256 reserveIn,
        uint256 reserveOut
    ) external pure returns (uint256 amountOut) {
        require(amountIn > 0, "UniswapV2Library: INSUFFICIENT_INPUT_AMOUNT");
        require(
            reserveIn > 0 && reserveOut > 0,
            "UniswapV2Library: INSUFFICIENT_LIQUIDITY"
        );
        uint256 amountInWithFee = amountIn * 997;
        uint256 numerator = amountInWithFee * reserveOut;
        uint256 denominator = reserveIn * 1000 + amountInWithFee;
        amountOut = numerator / denominator;
    }
}