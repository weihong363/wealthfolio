import { AlternativeAssetKind } from "@/lib/types";
import * as z from "zod";

// Metal types (labels are i18n keys resolved at render time)
export const METAL_TYPES = [
  { value: "gold", labelKey: "holdings.metalGold" },
  { value: "silver", labelKey: "holdings.metalSilver" },
  { value: "platinum", labelKey: "holdings.metalPlatinum" },
  { value: "palladium", labelKey: "holdings.metalPalladium" },
] as const;

// Weight units (labels are i18n keys resolved at render time)
export const WEIGHT_UNITS = [
  { value: "oz", labelKey: "holdings.unitTroyOunce" },
  { value: "g", labelKey: "holdings.unitGram" },
  { value: "kg", labelKey: "holdings.unitKilogram" },
] as const;

// Liability types (labels are i18n keys resolved at render time)
export const LIABILITY_TYPES = [
  { value: "mortgage", labelKey: "holdings.liabilityMortgage" },
  { value: "auto_loan", labelKey: "holdings.liabilityAutoLoan" },
  { value: "student_loan", labelKey: "holdings.liabilityStudentLoan" },
  { value: "credit_card", labelKey: "holdings.liabilityCreditCard" },
  { value: "personal_loan", labelKey: "holdings.liabilityPersonalLoan" },
  { value: "heloc", labelKey: "holdings.liabilityHeloc" },
  { value: "other", labelKey: "holdings.other" },
] as const;

// Asset type options (labels are i18n keys resolved at render time)
export const ASSET_KIND_OPTIONS = [
  { value: AlternativeAssetKind.PROPERTY, labelKey: "holdings.assetTypeProperty" },
  { value: AlternativeAssetKind.VEHICLE, labelKey: "holdings.assetTypeVehicle" },
  { value: AlternativeAssetKind.COLLECTIBLE, labelKey: "holdings.assetTypeCollectible" },
  { value: AlternativeAssetKind.PRECIOUS_METAL, labelKey: "holdings.assetTypePrecious" },
  { value: AlternativeAssetKind.LIABILITY, labelKey: "holdings.assetTypeLiability" },
  { value: AlternativeAssetKind.OTHER, labelKey: "holdings.assetTypeOther" },
] as const;

// Zod schema for the quick add form
export const alternativeAssetQuickAddSchema = z
  .object({
    // Asset type
    kind: z.enum([
      AlternativeAssetKind.PROPERTY,
      AlternativeAssetKind.VEHICLE,
      AlternativeAssetKind.COLLECTIBLE,
      AlternativeAssetKind.PRECIOUS_METAL,
      AlternativeAssetKind.LIABILITY,
      AlternativeAssetKind.OTHER,
    ]),

    // Common fields
    name: z.string().min(1, "Name is required").max(100, "Name must be less than 100 characters"),
    currency: z.string().min(1, "Currency is required"),
    quantity: z.coerce
      .number({
        required_error: "Please enter a valid quantity.",
        invalid_type_error: "Quantity must be a number.",
      })
      .positive("Quantity must be greater than 0"),
    currentValue: z.coerce
      .number({
        required_error: "Please enter a valid value.",
        invalid_type_error: "Value must be a number.",
      })
      .min(0, "Value cannot be negative"),
    valueDate: z.date({
      required_error: "Value date is required",
    }),

    // Property-specific: has mortgage checkbox
    hasMortgage: z.boolean().optional(),

    // Precious metal-specific fields
    metalType: z.enum(["gold", "silver", "platinum", "palladium"]).optional(),
    weightUnit: z.enum(["oz", "g", "kg"]).optional(),

    // Liability-specific fields
    liabilityType: z
      .enum([
        "mortgage",
        "auto_loan",
        "student_loan",
        "credit_card",
        "personal_loan",
        "heloc",
        "other",
      ])
      .optional(),
    linkedAssetId: z.string().optional(),
  })
  .refine(
    (data) => {
      // Precious metals require metal type and weight unit
      if (data.kind === AlternativeAssetKind.PRECIOUS_METAL) {
        return !!data.metalType && !!data.weightUnit;
      }
      return true;
    },
    {
      message: "Metal type and unit are required for precious metals",
      path: ["metalType"],
    },
  );

export type AlternativeAssetQuickAddFormValues = z.infer<typeof alternativeAssetQuickAddSchema>;

// Default form values
export const getDefaultFormValues = (): AlternativeAssetQuickAddFormValues => ({
  kind: AlternativeAssetKind.PROPERTY,
  name: "",
  currency: "USD",
  quantity: 1,
  currentValue: 0,
  valueDate: new Date(),
  hasMortgage: false,
  metalType: undefined,
  weightUnit: "oz",
  liabilityType: undefined,
  linkedAssetId: undefined,
});
