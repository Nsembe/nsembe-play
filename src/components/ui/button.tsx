import * as React from "react";
import { Slot } from "radix-ui";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

const buttonVariants = cva(
  "inline-flex shrink-0 items-center justify-center gap-2 whitespace-nowrap font-medium transition-[color,background-color,transform,opacity] duration-150 disabled:pointer-events-none disabled:opacity-40 active:scale-[0.97] [&_svg]:pointer-events-none [&_svg]:shrink-0",
  {
    variants: {
      variant: {
        primary: "bg-accent text-forest hover:bg-accent-hover font-semibold",
        secondary: "bg-white/10 text-fg hover:bg-white/15 backdrop-blur",
        ghost: "text-muted hover:bg-white/[0.06] hover:text-fg",
        outline: "border border-line-strong text-fg hover:bg-white/[0.06]",
        danger: "bg-danger/15 text-danger hover:bg-danger/25",
        link: "text-accent hover:underline underline-offset-4 px-0",
      },
      size: {
        sm: "h-8 rounded-md px-3 text-[13px] [&_svg]:size-4",
        md: "h-10 rounded-lg px-4 text-sm [&_svg]:size-[18px]",
        lg: "h-12 rounded-full px-6 text-[15px] [&_svg]:size-5",
        icon: "size-9 rounded-full [&_svg]:size-[18px]",
        "icon-sm": "size-8 rounded-full [&_svg]:size-4",
        "icon-lg": "size-12 rounded-full [&_svg]:size-6",
      },
    },
    defaultVariants: { variant: "secondary", size: "md" },
  },
);

export interface ButtonProps extends React.ComponentProps<"button">, VariantProps<typeof buttonVariants> {
  asChild?: boolean;
}

export function Button({ className, variant, size, asChild = false, ...props }: ButtonProps) {
  const Comp = asChild ? Slot.Root : "button";
  return <Comp className={cn(buttonVariants({ variant, size }), className)} {...props} />;
}

export { buttonVariants };
