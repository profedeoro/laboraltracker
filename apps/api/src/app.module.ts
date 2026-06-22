import { Module } from '@nestjs/common';
import { ConfigModule } from '@nestjs/config';
import { AppController } from './app.controller';
import { PrismaModule } from './database/prisma.module';
// TODO(Phase 1b Task 7): enable AuthModule
// import { AuthModule } from './modules/auth/auth.module';
import { validateEnv } from './config/env.validation';

@Module({
  imports: [
    ConfigModule.forRoot({ isGlobal: true, validate: validateEnv }),
    PrismaModule,
    // TODO(Phase 1b Task 7): enable AuthModule
    // AuthModule,
  ],
  controllers: [AppController],
  providers: [],
})
export class AppModule {}
