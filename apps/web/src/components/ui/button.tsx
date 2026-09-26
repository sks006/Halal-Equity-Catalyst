import * as React from "react";
import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";

import { cn } from "../../lib/utils";

const buttonVariants = cva(
  "inline-flex items-center justify-center whitespace-nowrap rounded-lg text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-emerald-500 focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50",
  {
    variants: {
      variant: {
        // Primary action: green
        default:
          "bg-emerald-600 text-white shadow-sm hover:bg-emerald-700",
        // Secondary action: neutral/slate
        secondary:
          "border border-slate-300 bg-white text-slate-700 shadow-sm hover:bg-slate-50",
        outline:
          "border border-slate-300 bg-white text-slate-700 shadow-sm hover:bg-slate-50",
        // Danger action: red only when dangerous or failed
        destructive:
          "bg-rose-600 text-white shadow-sm hover:bg-rose-700",
        ghost:
          "hover:bg-slate-100 hover:text-slate-900 text-slate-700",
        link:
          "text-emerald-700 underline-offset-4 hover:underline",
        emerald:
          "bg-emerald-600 text-white shadow-sm hover:bg-emerald-700",
      },
      size: {
        default: "h-9 px-4 py-2",
        sm: "h-8 rounded-lg px-3 text-xs",
        lg: "h-11 rounded-lg px-8 text-sm font-semibold",
        icon: "h-9 w-9",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  }
);

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  asChild?: boolean;
}

const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, asChild = false, ...props }, ref) => {
    const Comp = asChild ? Slot : "button";
    return (
      <Comp
        className={cn(buttonVariants({ variant, size, className }))}
        ref={ref}
        {...props}
      />
    );
  }
);
Button.displayName = "Button";

export { Button, buttonVariants };
