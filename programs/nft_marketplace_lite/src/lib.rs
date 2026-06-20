use anchor_lang::error_code;
use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{CloseAccount, Mint, Token, TokenAccount};

declare_id!("3cWJgR38gF3iXLi5JbSRCr9e6Q1BxbV5eGaS2Sgk3DHJ");

#[error_code]
pub enum ErrorCode {
    #[msg("Price must be greater than zero")]
    InvalidPrice,
    #[msg("Price must be equal to the listing price")]
    InvalidBayerPrice,
    #[msg("Account is not an owner")]
    InvalidSellerAccount,
}

#[program]
pub mod nft_marketplace_lite {
    use super::*;
    use anchor_spl::token;
    pub fn list_nft(ctx: Context<ListNft>, price: u64) -> Result<()> {
        require!(price > 0, ErrorCode::InvalidPrice);
        let listing = &mut ctx.accounts.listing;
        listing.seller = ctx.accounts.seller.key();
        listing.mint = ctx.accounts.mint.key();
        listing.price = price;
        listing.bump = ctx.bumps.listing;

        // Transfer the NFT from the seller to the vault
        let cpi_accounts = anchor_spl::token::Transfer {
            from: ctx.accounts.seller_nft_account.to_account_info(),
            to: ctx.accounts.vault.to_account_info(),
            authority: ctx.accounts.seller.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, 1)?;

        Ok(())
    }

    pub fn buy_nft(ctx: Context<BuyNft>, price: u64) -> Result<()> {
        let listing = &ctx.accounts.listing;
        require!(price == listing.price, ErrorCode::InvalidBayerPrice);

        // Transfer the payment from the buyer to the seller
        let cpi_accounts = system_program::Transfer {
            from: ctx.accounts.buyer.to_account_info(),
            to: ctx.accounts.seller.to_account_info(),
        };
        let transfer_ctx =
            CpiContext::new(ctx.accounts.system_program.to_account_info(), cpi_accounts);
        system_program::transfer(transfer_ctx, price)?;

        // Transfer the NFT from the vault to the buyer
        let cpi_accounts = token::Transfer {
            from: ctx.accounts.vault.to_account_info(),
            to: ctx.accounts.buyer_nft_account.to_account_info(),
            authority: ctx.accounts.listing.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let seeds = &[
            b"listing",
            listing.seller.as_ref(),
            listing.mint.as_ref(),
            &[listing.bump],
        ];
        let signer_seeds = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        token::transfer(cpi_ctx, 1)?;

        let cpi_accounts = CloseAccount {
            account: ctx.accounts.vault.to_account_info(),
            destination: ctx.accounts.seller.to_account_info(),
            authority: ctx.accounts.listing.to_account_info(),
        };

        let close_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );

        token::close_account(close_ctx)?;

        Ok(())
    }

    pub fn cancel_listing(ctx: Context<CancelListing>) -> Result<()> {
        let listing = &ctx.accounts.listing;

        // Transfer the NFT back from the vault to the seller
        let cpi_accounts = token::Transfer {
            from: ctx.accounts.vault.to_account_info(),
            to: ctx.accounts.seller_nft_account.to_account_info(),
            authority: ctx.accounts.listing.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let seeds = &[
            b"listing",
            listing.seller.as_ref(),
            listing.mint.as_ref(),
            &[listing.bump],
        ];
        let signer_seeds = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        token::transfer(cpi_ctx, 1)?;

        let cpi_accounts = CloseAccount {
            account: ctx.accounts.vault.to_account_info(),
            destination: ctx.accounts.seller.to_account_info(),
            authority: ctx.accounts.listing.to_account_info(),
        };

        let close_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );

        token::close_account(close_ctx)?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct ListNft<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,
    #[account(
       constraint = mint.decimals == 0,
       constraint = mint.supply == 1
    )]
    pub mint: Account<'info, Mint>,
    #[account(
        init,
        payer = seller,
        space = 8 + Listing::INIT_SPACE,
        seeds = [b"listing", seller.key().as_ref(), mint.key().as_ref()],
        bump
    )]
    pub listing: Account<'info, Listing>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = seller,
    )]
    pub seller_nft_account: Account<'info, TokenAccount>,
    #[account(
        init,
        payer = seller,
        associated_token::mint = mint,
        associated_token::authority = listing,
    )]
    pub vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct BuyNft<'info> {
    #[account(
        mut,
        seeds = [b"listing", listing.seller.as_ref(), mint.key().as_ref()],
        close = seller,
        bump = listing.bump,
    )]
    pub listing: Account<'info, Listing>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = listing,
    )]
    pub vault: Account<'info, TokenAccount>,
    #[account(mut, address = listing.seller)]
    /// CHECK: checked by address = listing.seller
    pub seller: UncheckedAccount<'info>,
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = buyer,
    )]
    pub buyer_nft_account: Account<'info, TokenAccount>,
    #[account(address = listing.mint)]
    pub mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CancelListing<'info> {
    #[account(
        mut,
        seeds = [b"listing", listing.seller.as_ref(), mint.key().as_ref()],
        close = seller,
        bump = listing.bump,
    )]
    pub listing: Account<'info, Listing>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = listing,
    )]
    pub vault: Account<'info, TokenAccount>,
    #[account(mut, address = listing.seller)]
    pub seller: Signer<'info>,
    #[account(
    mut,
    associated_token::mint = mint,
    associated_token::authority = seller,
    )]
    pub seller_nft_account: Account<'info, TokenAccount>,
    /// CHECK: checked by address = listing.mint
    #[account(address = listing.mint)]
    pub mint: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
}

#[account]
#[derive(InitSpace)]
pub struct Listing {
    pub seller: Pubkey,
    pub mint: Pubkey,
    pub price: u64,
    pub bump: u8,
}
