import { IsString, MinLength } from 'class-validator';

export class SwitchCompanyDto {
  @IsString()
  @MinLength(1)
  companyId!: string;
}
